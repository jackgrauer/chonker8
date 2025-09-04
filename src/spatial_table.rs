use ropey::Rope;
use eframe::egui;

/// Spatial table reconstructed from Alto XML coordinates
#[derive(Debug, Clone)]
pub struct SpatialTable {
    /// Table structure with rows and columns
    pub cells: Vec<Vec<TableCell>>,
    /// Column definitions with HPOS positions
    pub columns: Vec<TableColumn>,
    /// Original page dimensions for scaling
    pub page_width: f32,
    pub page_height: f32,
    /// Bounding box of entire table
    pub bbox: BoundingBox,
    /// Table metadata
    pub table_id: String,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    /// Cell content as rope for efficient editing
    pub rope: Rope,
    /// Display text (cached from rope)
    pub text: String,
    /// Original Alto coordinates 
    pub hpos: f32,
    pub vpos: f32,
    pub width: f32,
    pub height: f32,
    /// Cell formatting
    pub style: CellStyle,
    /// Column and row indices
    pub col_index: usize,
    pub row_index: usize,
}

#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Column header text
    pub header: String,
    /// HPOS coordinate for this column
    pub hpos: f32,
    /// Column width (calculated from data)
    pub width: f32,
    /// Column alignment based on content
    pub alignment: ColumnAlignment,
}

#[derive(Debug, Clone)]
pub enum ColumnAlignment {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone)]
pub struct CellStyle {
    pub color: egui::Color32,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_header: bool,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// Alto element with spatial coordinates
#[derive(Debug, Clone)]
pub struct AltoElement {
    pub content: String,
    pub hpos: f32,
    pub vpos: f32,
    pub width: f32,
    pub height: f32,
    pub style_refs: String,
}

impl SpatialTable {
    /// Create spatial table from Alto XML elements using coordinate clustering
    pub fn from_alto_elements(elements: Vec<AltoElement>, page_width: f32, page_height: f32) -> Self {
        let mut table = SpatialTable {
            cells: Vec::new(),
            columns: Vec::new(),
            page_width,
            page_height,
            bbox: BoundingBox { left: 0.0, top: 0.0, width: 0.0, height: 0.0 },
            table_id: format!("table_{}", chrono::Utc::now().timestamp_millis()),
        };
        
        if elements.is_empty() {
            return table;
        }
        
        // Step 1: Detect columns by clustering HPOS coordinates
        let column_positions = Self::detect_columns(&elements);
        
        // Step 2: Group elements by rows (VPOS clustering)
        let row_groups = Self::group_by_rows(&elements);
        
        // Step 3: Create column definitions
        table.columns = column_positions.iter().enumerate().map(|(i, &hpos)| {
            TableColumn {
                header: format!("Col {}", i + 1),
                hpos,
                width: 100.0, // Will be calculated later
                alignment: ColumnAlignment::Left,
            }
        }).collect();
        
        // Step 4: Build table cells matrix
        table.cells = Vec::new();
        
        for (row_idx, row_elements) in row_groups.iter().enumerate() {
            let mut row_cells = Vec::new();
            
            // Create cells for each column
            for (col_idx, column) in table.columns.iter().enumerate() {
                // Find element closest to this column's HPOS
                let cell_element = Self::find_closest_element(row_elements, column.hpos);
                
                let cell = if let Some(element) = cell_element {
                    TableCell {
                        text: element.content.clone(),
                        rope: Rope::from_str(&element.content),
                        hpos: element.hpos,
                        vpos: element.vpos,
                        width: element.width,
                        height: element.height,
                        style: Self::determine_cell_style(&element, row_idx == 0),
                        col_index: col_idx,
                        row_index: row_idx,
                    }
                } else {
                    // Empty cell
                    TableCell {
                        text: String::new(),
                        rope: Rope::from_str(""),
                        hpos: column.hpos,
                        vpos: row_elements.first().map(|e| e.vpos).unwrap_or(0.0),
                        width: 0.0,
                        height: 12.0,
                        style: CellStyle {
                            color: egui::Color32::from_rgb(180, 180, 180),
                            font_size: 12.0,
                            is_bold: false,
                            is_header: row_idx == 0,
                        },
                        col_index: col_idx,
                        row_index: row_idx,
                    }
                };
                
                row_cells.push(cell);
            }
            
            table.cells.push(row_cells);
        }
        
        // Step 5: Calculate table bounding box
        table.calculate_bbox();
        
        // Step 6: Determine column alignments and headers from content
        table.analyze_content();
        
        table
    }
    
    /// Detect column positions by clustering HPOS coordinates
    fn detect_columns(elements: &[AltoElement]) -> Vec<f32> {
        if elements.is_empty() {
            return Vec::new();
        }
        
        // Collect all unique HPOS values
        let mut hpos_values: Vec<f32> = elements.iter().map(|e| e.hpos).collect();
        hpos_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // Cluster nearby HPOS values (within 20 units)
        let mut columns = Vec::new();
        let tolerance = 20.0;
        
        let mut current_cluster = vec![hpos_values[0]];
        
        for &hpos in &hpos_values[1..] {
            if let Some(&last) = current_cluster.last() {
                if (hpos - last).abs() <= tolerance {
                    current_cluster.push(hpos);
                } else {
                    // Finish current cluster and start new one
                    let avg = current_cluster.iter().sum::<f32>() / current_cluster.len() as f32;
                    columns.push(avg);
                    current_cluster = vec![hpos];
                }
            }
        }
        
        // Add final cluster
        if !current_cluster.is_empty() {
            let avg = current_cluster.iter().sum::<f32>() / current_cluster.len() as f32;
            columns.push(avg);
        }
        
        columns
    }
    
    /// Group elements by rows using VPOS clustering
    fn group_by_rows(elements: &[AltoElement]) -> Vec<Vec<AltoElement>> {
        if elements.is_empty() {
            return Vec::new();
        }
        
        // Sort elements by VPOS
        let mut sorted_elements = elements.to_vec();
        sorted_elements.sort_by(|a, b| a.vpos.partial_cmp(&b.vpos).unwrap());
        
        let mut row_groups = Vec::new();
        let tolerance = 15.0; // VPOS tolerance for same row
        
        let mut current_row = vec![sorted_elements[0].clone()];
        let mut current_vpos = sorted_elements[0].vpos;
        
        for element in &sorted_elements[1..] {
            if (element.vpos - current_vpos).abs() <= tolerance {
                current_row.push(element.clone());
            } else {
                // Sort row by HPOS for left-to-right order
                current_row.sort_by(|a, b| a.hpos.partial_cmp(&b.hpos).unwrap());
                row_groups.push(current_row);
                
                current_row = vec![element.clone()];
                current_vpos = element.vpos;
            }
        }
        
        // Add final row
        if !current_row.is_empty() {
            current_row.sort_by(|a, b| a.hpos.partial_cmp(&b.hpos).unwrap());
            row_groups.push(current_row);
        }
        
        row_groups
    }
    
    /// Find element closest to target HPOS position
    fn find_closest_element(elements: &[AltoElement], target_hpos: f32) -> Option<&AltoElement> {
        elements.iter()
            .min_by(|a, b| {
                let dist_a = (a.hpos - target_hpos).abs();
                let dist_b = (b.hpos - target_hpos).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .filter(|element| (element.hpos - target_hpos).abs() <= 50.0) // Max distance tolerance
    }
    
    /// Determine cell style from Alto element
    fn determine_cell_style(element: &AltoElement, is_header: bool) -> CellStyle {
        let is_bold = element.style_refs.to_lowercase().contains("bold") || is_header;
        
        CellStyle {
            color: if is_header {
                egui::Color32::from_rgb(220, 220, 220)
            } else {
                egui::Color32::from_rgb(180, 180, 180)
            },
            font_size: if is_header { 14.0 } else { 12.0 },
            is_bold,
            is_header,
        }
    }
    
    /// Calculate table bounding box from all cells
    fn calculate_bbox(&mut self) {
        if self.cells.is_empty() {
            return;
        }
        
        let mut min_left = f32::MAX;
        let mut max_right = f32::MIN;
        let mut min_top = f32::MAX;
        let mut max_bottom = f32::MIN;
        
        for row in &self.cells {
            for cell in row {
                min_left = min_left.min(cell.hpos);
                max_right = max_right.max(cell.hpos + cell.width);
                min_top = min_top.min(cell.vpos);
                max_bottom = max_bottom.max(cell.vpos + cell.height);
            }
        }
        
        self.bbox = BoundingBox {
            left: min_left,
            top: min_top,
            width: max_right - min_left,
            height: max_bottom - min_top,
        };
    }
    
    /// Analyze content to determine column properties
    fn analyze_content(&mut self) {
        for (col_idx, column) in self.columns.iter_mut().enumerate() {
            // Determine alignment from content patterns
            let mut numeric_count = 0;
            let mut total_count = 0;
            
            for row in &self.cells {
                if let Some(cell) = row.get(col_idx) {
                    if !cell.text.trim().is_empty() {
                        total_count += 1;
                        if cell.text.trim().parse::<f64>().is_ok() || 
                           cell.text.contains("$") || 
                           cell.text.contains("%") {
                            numeric_count += 1;
                        }
                    }
                }
            }
            
            // Determine alignment
            if total_count > 0 && numeric_count as f32 / total_count as f32 > 0.7 {
                column.alignment = ColumnAlignment::Right;
            } else {
                column.alignment = ColumnAlignment::Left;
            }
            
            // Set header from first row if available
            if let Some(first_row) = self.cells.first() {
                if let Some(header_cell) = first_row.get(col_idx) {
                    if !header_cell.text.trim().is_empty() {
                        column.header = header_cell.text.trim().to_string();
                    }
                }
            }
        }
    }
    
    /// Render table using egui with proper spatial layout
    pub fn render(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        
        // Table title
        ui.heading(format!("📊 Spatial Table ({}x{})", self.cells.len(), self.columns.len()));
        
        // Table statistics
        ui.horizontal(|ui| {
            ui.label(format!("🎯 Position: ({:.0}, {:.0})", self.bbox.left, self.bbox.top));
            ui.separator();
            ui.label(format!("📐 Size: {:.0}×{:.0}", self.bbox.width, self.bbox.height));
        });
        
        ui.separator();
        
        // Scrollable table area
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Table implementation
                egui::Grid::new("spatial_table")
                    .num_columns(self.columns.len())
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header row
                        for column in &self.columns {
                            ui.colored_label(
                                egui::Color32::from_rgb(220, 220, 220),
                                egui::RichText::new(&column.header).strong().size(14.0)
                            );
                        }
                        ui.end_row();
                        
                        // Data rows
                        for row in &mut self.cells {
                            for cell in row {
                                let mut cell_text = cell.text.clone();
                                
                                let text_edit = egui::TextEdit::singleline(&mut cell_text)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(120.0);
                                
                                let response = ui.add(text_edit);
                                
                                if response.changed() {
                                    cell.text = cell_text.clone();
                                    cell.rope = Rope::from_str(&cell_text);
                                    changed = true;
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        
        changed
    }
    
    /// Export table as CSV
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        
        // Header row
        let headers: Vec<String> = self.columns.iter().map(|col| col.header.clone()).collect();
        csv.push_str(&headers.join(","));
        csv.push('\n');
        
        // Data rows  
        for row in &self.cells {
            let row_data: Vec<String> = row.iter().map(|cell| {
                // Escape CSV special characters
                if cell.text.contains(',') || cell.text.contains('"') || cell.text.contains('\n') {
                    format!("\"{}\"", cell.text.replace("\"", "\"\""))
                } else {
                    cell.text.clone()
                }
            }).collect();
            csv.push_str(&row_data.join(","));
            csv.push('\n');
        }
        
        csv
    }
    
    /// Get cell at specific position
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        self.cells.get(row)?.get(col)
    }
    
    /// Get cell at specific position (mutable)
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Option<&mut TableCell> {
        self.cells.get_mut(row)?.get_mut(col)
    }
}