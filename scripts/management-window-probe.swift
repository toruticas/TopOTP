// Synthetic E2E window only. No vault access, screenshots or personal content.
import AppKit
import CoreGraphics
import Foundation

let observer = NSApplication.shared
observer.setActivationPolicy(.prohibited)

guard CommandLine.arguments.count == 2,
      CommandLine.arguments[1].hasSuffix("/.artifacts/e2e-target/debug/topotp")
else { fatalError("Only the isolated E2E executable is allowed") }
let executable = URL(fileURLWithPath: CommandLine.arguments[1]).standardizedFileURL
guard let target = NSWorkspace.shared.runningApplications.first(where: { $0.executableURL?.standardizedFileURL == executable }),
      let finder = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").first
else { fatalError("Run the synthetic E2E app with Finder available first") }

func windows() -> Set<Int> {
    let entries = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] ?? []
    return Set(entries.compactMap { entry in
        guard entry[kCGWindowOwnerPID as String] as? Int32 == target.processIdentifier,
              entry[kCGWindowLayer as String] as? Int == 0,
              let bounds = entry[kCGWindowBounds as String] as? [String: CGFloat],
              bounds["Width", default: 0] > 200, bounds["Height", default: 0] > 200
        else { return nil }
        return entry[kCGWindowNumber as String] as? Int
    })
}
func waitForActivation(_ application: NSRunningApplication) {
    let deadline = Date().addingTimeInterval(3)
    while NSWorkspace.shared.frontmostApplication?.processIdentifier != application.processIdentifier && Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
}
target.activate(options: [.activateIgnoringOtherApps])
waitForActivation(target)
let targetWasActive = NSWorkspace.shared.frontmostApplication?.processIdentifier == target.processIdentifier
let before = windows()
defer { target.activate(options: [.activateIgnoringOtherApps]) }
let activationRequested = finder.activate(options: [.activateIgnoringOtherApps])
waitForActivation(finder)
RunLoop.current.run(until: Date().addingTimeInterval(0.4))
let after = windows()
let result: [String: Any] = [
    "syntheticOnly": true,
    "activationRequested": activationRequested,
    "targetWasActive": targetWasActive,
    "activeApplication": NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "unknown",
    "otherAppActive": NSWorkspace.shared.frontmostApplication?.processIdentifier == finder.processIdentifier,
    "managementStayedOnscreen": !before.isEmpty && before.isSubset(of: after),
]
let bytes = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
print(String(decoding: bytes, as: UTF8.self))
