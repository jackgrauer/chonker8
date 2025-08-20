import Vision
import Foundation

let request = VNRecognizeTextRequest { request, error in
    if let error = error {
        print("Vision error: \(error)")
    } else {
        print("Vision framework is accessible")
    }
}
print("Vision test complete")
