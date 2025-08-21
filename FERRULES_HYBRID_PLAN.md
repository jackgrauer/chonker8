# Ferrules Hybrid Approach: Layout Only, Text from pdftotext

## The Strategy: Best of Both Worlds

### Use Ferrules For:
✅ **Bounding boxes** - Precise x0, y0, x1, y1 coordinates
✅ **Text block segmentation** - Knows where each paragraph/cell is
✅ **Layout structure** - Understands document organization
✅ **Page dimensions** - Accurate width/height

### Use pdftotext For:
✅ **Actual text content** - Accurate, no gibberish
✅ **Character extraction** - Reliable text
✅ **Technical accuracy** - Handles tables, numbers, special chars

## Implementation Approach

```rust
// Step 1: Get layout from Ferrules
let ferrules_output = run_ferrules(pdf_path);
let blocks = ferrules_output.blocks; // Has bbox but bad text

// Step 2: Get clean text from pdftotext  
let clean_text = extract_with_pdftotext(pdf_path);

// Step 3: Map clean text to Ferrules layout
for block in blocks {
    let bbox = block.bbox; // Keep the accurate position
    
    // Find the text that belongs in this bbox from pdftotext
    let actual_text = find_text_in_region(clean_text, bbox);
    
    // Replace Ferrules' gibberish with real text
    block.text = actual_text;
}
```

## Benefits

1. **Perfect layout preservation** - Ferrules knows exactly where everything is
2. **Accurate text** - pdftotext gives us the real content
3. **Table structure** - Can detect tables using Ferrules boxes, fill with pdftotext
4. **No more gibberish** - Never use Vision API text
5. **Best performance** - Both tools doing what they're best at

## Example: Table Detection

```rust
// Ferrules tells us there's a table at these coordinates
Table {
    bbox: { x0: 50, y0: 100, x1: 500, y1: 400 },
    cells: [
        Cell { bbox: { x0: 50, y0: 100, x1: 150, y1: 120 }, text: "Tahlst" }, // BAD
        Cell { bbox: { x0: 150, y0: 100, x1: 250, y1: 120 }, text: "Soll" },  // BAD
    ]
}

// We keep the structure but replace with pdftotext content
Table {
    bbox: { x0: 50, y0: 100, x1: 500, y1: 400 },  // KEEP
    cells: [
        Cell { bbox: { x0: 50, y0: 100, x1: 150, y1: 120 }, text: "Table 6" }, // FIXED
        Cell { bbox: { x0: 150, y0: 100, x1: 250, y1: 120 }, text: "Soil" },   // FIXED
    ]
}
```

## Implementation Steps

1. **Modify ferrules_extraction.rs**
   - Keep bbox extraction
   - Ignore text field from Ferrules
   - Add pdftotext integration

2. **Create layout_mapper.rs**
   ```rust
   pub fn map_text_to_layout(
       ferrules_blocks: Vec<FerrulesBlock>,
       pdftotext_content: String,
   ) -> Vec<BlockWithRealText>
   ```

3. **Region matching algorithm**
   - Use bbox coordinates to find text position
   - Match pdftotext lines to Ferrules blocks
   - Handle edge cases (split words, hyphenation)

## This Solves Everything!

- ✅ No more gibberish
- ✅ Perfect layout preservation  
- ✅ Table structure maintained
- ✅ Fast and reliable
- ✅ Works on all PDFs