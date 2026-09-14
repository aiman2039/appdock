// Rebuild with: xcrun swift scripts/dmg-background.swift assets/installer/background.png
import AppKit
let size = NSSize(width: 660, height: 400)
let image = NSImage(size: size)
image.lockFocus()
NSColor(calibratedWhite: 0.96, alpha: 1).setFill()
NSRect(origin: .zero, size: size).fill()
func text(_ value: String, y: CGFloat, size: CGFloat, color: NSColor) {
    let style = NSMutableParagraphStyle()
    style.alignment = .center
    (value as NSString).draw(in: NSRect(x: 20, y: y, width: 620, height: 45), withAttributes: [
        .font: NSFont.systemFont(ofSize: size, weight: size > 20 ? .semibold : .regular),
        .foregroundColor: color, .paragraphStyle: style
    ])
}
text("Install AppDock", y: 323, size: 28, color: .labelColor)
text("Drag AppDock into Applications", y: 281, size: 16, color: .secondaryLabelColor)
let arrow = NSBezierPath()
arrow.move(to: NSPoint(x: 285, y: 202))
arrow.line(to: NSPoint(x: 375, y: 202))
arrow.move(to: NSPoint(x: 359, y: 218))
arrow.line(to: NSPoint(x: 375, y: 202))
arrow.line(to: NSPoint(x: 359, y: 186))
arrow.lineWidth = 4
NSColor.systemBlue.setStroke()
arrow.stroke()
text("Then eject this disk and open AppDock from Applications.", y: 48, size: 14, color: .secondaryLabelColor)
image.unlockFocus()
let bitmap = NSBitmapImageRep(data: image.tiffRepresentation!)!
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
