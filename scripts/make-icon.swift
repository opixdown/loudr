// Turns the designed icon (opaque white page, baked shadow) into an app icon
// with transparent corners, and draws the menu bar template icon.
//
//   swift scripts/make-icon.swift assets/icon-original.png assets/icon.png \
//       src-tauri/icons/tray-template.png
//   npm run tauri icon assets/icon.png
import AppKit

let args = CommandLine.arguments
let src = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: args[1])))!
let N = 1024

func inside(_ x: Int, _ y: Int) -> Bool {
    let c = src.colorAt(x: x, y: y)!
    return (c.redComponent - c.blueComponent) > 0.015 || c.redComponent < 0.5
}

// Outline of the rounded square, traced from the clean bottom half and
// mirrored to the top (the top edge has a white highlight that can't be
// told apart from the page).
var left: [CGPoint] = [], right: [CGPoint] = []
for y in 512..<N {
    guard let a = (0..<N).first(where: { inside($0, y) }),
          let b = (0..<N).reversed().first(where: { inside($0, y) }) else { break }
    left.append(CGPoint(x: Double(a), y: Double(y)))
    right.append(CGPoint(x: Double(b) + 1, y: Double(y)))
}
let mirror = { (p: CGPoint) in CGPoint(x: p.x, y: 1023 - p.y) }
// Image coordinates (y down): top-left going down the left side, then up the right.
let outline = left.reversed().map(mirror) + left + right.reversed() + right.map(mirror)

func context(_ size: Int) -> CGContext {
    CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
              space: CGColorSpace(name: CGColorSpace.sRGB)!,
              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
}
func save(_ ctx: CGContext, _ path: String) {
    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

// ---- App icon ----
let ctx = context(N)
ctx.translateBy(x: 0, y: CGFloat(N)); ctx.scaleBy(x: 1, y: -1) // y down, like the image
let path = CGMutablePath()
path.addLines(between: outline); path.closeSubpath()
// Pull the outline in by ~1.5 px so no white page pixels survive at the edge.
var inset = CGAffineTransform(translationX: 512, y: 512).scaledBy(x: 0.9965, y: 0.9965).translatedBy(x: -512, y: -512)
let shape = path.copy(using: &inset)!

ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 22, color: CGColor(gray: 0, alpha: 0.22))
ctx.setFillColor(CGColor(srgbRed: 0.92, green: 0.91, blue: 0.89, alpha: 1))
ctx.addPath(shape); ctx.fillPath()
ctx.restoreGState()

ctx.addPath(shape); ctx.clip()
ctx.scaleBy(x: 1, y: -1); ctx.translateBy(x: 0, y: -CGFloat(N)) // images draw y up
ctx.draw(src.cgImage!, in: CGRect(x: 0, y: 0, width: N, height: N))
save(ctx, args[2])

// ---- Menu bar template icon: just the bars, black on transparent ----
let T = 88
let bars: [(x: Double, top: Double, bottom: Double)] =
    [(306, 452, 572), (396, 402, 622), (486, 342, 682), (576, 402, 622), (666, 452, 572)]
let barWidth = 52.0, scale = 76.0 / 412.0
let t = context(T)
t.setFillColor(CGColor(gray: 0, alpha: 1))
for bar in bars {
    let w = barWidth * scale, h = (bar.bottom - bar.top) * scale
    let x = (Double(T) - 412 * scale) / 2 + (bar.x - 306) * scale
    let rect = CGRect(x: x, y: (Double(T) - h) / 2, width: w, height: h)
    t.addPath(CGPath(roundedRect: rect, cornerWidth: w / 2, cornerHeight: w / 2, transform: nil))
    t.fillPath()
}
save(t, args[3])
