import AppKit
import CoreImage

// Public RFC/sample accounts, displayed only for a manual ScreenCaptureKit test.
// No vault, capture, network, clipboard or image-file access.
@main
@MainActor
struct NativeQrFixture {
    static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 980, height: 570),
                              styleMask: [.titled, .closable, .miniaturizable],
                              backing: .buffered, defer: false)
        window.title = "TopOTP — QRs públicos para teste"
        window.isReleasedWhenClosed = false
        let root = NSStackView()
        root.orientation = .vertical
        root.spacing = 24
        root.edgeInsets = NSEdgeInsets(top: 24, left: 28, bottom: 24, right: 28)
        root.addArrangedSubview(NSTextField(labelWithString: "Contas sintéticas públicas — selecione um QR pelo scanner"))
        let row = NSStackView()
        row.orientation = .horizontal
        row.distribution = .fillEqually
        row.spacing = 48
        for (label, uri) in [
            ("TOTP público • SHA1 • 8 dígitos", "otpauth://totp/QR%20Sintetico:Teste%20TOTP?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=QR%20Sintetico&digits=8"),
            ("HOTP público • próximo contador 7", "otpauth://hotp/QR%20Sintetico:Teste%20HOTP?secret=JBSWY3DPEHPK3PXP&counter=7&issuer=QR%20Sintetico"),
        ] {
            let column = NSStackView()
            column.orientation = .vertical
            column.spacing = 16
            column.addArrangedSubview(NSTextField(labelWithString: label))
            guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return }
            filter.setValue(Data(uri.utf8), forKey: "inputMessage")
            filter.setValue("M", forKey: "inputCorrectionLevel")
            guard let image = filter.outputImage?.transformed(by: CGAffineTransform(scaleX: 8, y: 8)),
                  let cgImage = CIContext().createCGImage(image, from: image.extent) else { return }
            let view = NSImageView(image: NSImage(cgImage: cgImage, size: .zero))
            view.imageScaling = .scaleProportionallyUpOrDown
            view.setAccessibilityLabel(label)
            view.widthAnchor.constraint(equalToConstant: 380).isActive = true
            view.heightAnchor.constraint(equalToConstant: 380).isActive = true
            let background = NSBox()
            background.boxType = .custom
            background.fillColor = .white
            background.borderColor = .white
            background.contentViewMargins = NSSize(width: 16, height: 16)
            background.contentView = view
            column.addArrangedSubview(background)
            row.addArrangedSubview(column)
        }
        root.addArrangedSubview(row)
        window.contentView = root
        window.center()
        window.makeKeyAndOrderFront(nil)
        let token = NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification,
            object: window, queue: .main) { _ in NSApplication.shared.terminate(nil) }
        _ = token
        app.activate(ignoringOtherApps: true)
        app.run()
    }
}
