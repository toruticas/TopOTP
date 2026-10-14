import AppKit
import Foundation
import LocalAuthentication
import Security

// Isolated validation tool. This is not a replacement for the Rust vault.
// The only payload is public, synthetic data; existing Keychain items are never read.
@main
struct NativeKeychainProbe {
  static func main() {
    NSApplication.shared.setActivationPolicy(.accessory)
    if let offset = CommandLine.arguments.firstIndex(of: "--foreign-read"),
      CommandLine.arguments.count > offset + 1 {
      let service = CommandLine.arguments[offset + 1]
      guard service.hasPrefix("com.rafaelmariano.topotp.probe.") else { exit(4) }
      SecKeychainSetUserInteractionAllowed(false)
      let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: "synthetic-control",
        kSecUseDataProtectionKeychain as String: false,
        kSecReturnData as String: true,
      ]
      print(Int(SecItemCopyMatching(query as CFDictionary, nil)))
      return
    }
    let interactive = CommandLine.arguments.contains("--interactive")
    let dataProtection = CommandLine.arguments.contains("--data-protection")
    let id = UUID().uuidString
    let service = "com.rafaelmariano.topotp.probe.\(id)"
    var report: [String: Any] = [
      "bundleIdentifier": Bundle.main.bundleIdentifier ?? "unknown",
      "mode": interactive ? "interactive" : "noninteractive",
      "dataProtectionKeychain": dataProtection,
      "accessControl": dataProtection ? "userPresence" : "application-default-acl",
      "syntheticOnly": true,
      "authenticatedReadVerified": false,
    ]
    let context = LAContext()
    var policyError: NSError?
    report["deviceOwnerAuthenticationAvailable"] = context.canEvaluatePolicy(
      .deviceOwnerAuthentication, error: &policyError
    )
    report["touchIDAvailable"] = context.canEvaluatePolicy(
      .deviceOwnerAuthenticationWithBiometrics, error: &policyError
    )
    context.touchIDAuthenticationAllowableReuseDuration = 0
    context.interactionNotAllowed = !interactive

    let identity: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: "synthetic-control",
      kSecUseDataProtectionKeychain as String: dataProtection,
      kSecAttrSynchronizable as String: false,
    ]
    var create = identity
    if dataProtection {
      var accessError: Unmanaged<CFError>?
      guard let access = SecAccessControlCreateWithFlags(
        nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, .userPresence, &accessError
      ) else {
        report["accessControlCreated"] = false
        emit(report)
        return
      }
      report["accessControlCreated"] = true
      create[kSecAttrAccessControl as String] = access
    }
    create[kSecValueData as String] = Data("TopOTP public synthetic probe".utf8)
    let addStatus = SecItemAdd(create as CFDictionary, nil)
    report["addStatus"] = Int(addStatus)

    guard addStatus == errSecSuccess else {
      report["blockedByMissingEntitlement"] = addStatus == errSecMissingEntitlement
      emit(report)
      return
    }
    var read = identity
    read[kSecReturnData as String] = true
    read[kSecUseAuthenticationContext as String] = context
    if interactive && !dataProtection {
      let completed = DispatchSemaphore(value: 0)
      var accepted = false
      context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: "Validar desbloqueio do TopOTP") { success, _ in
        accepted = success
        completed.signal()
      }
      completed.wait()
      report["localAuthenticationSucceeded"] = accepted
      if !accepted {
        context.invalidate()
        report["cleanupStatus"] = Int(SecItemDelete(identity as CFDictionary))
        emit(report)
        return
      }
    }
    var item: CFTypeRef?
    let readStatus = SecItemCopyMatching(read as CFDictionary, &item)
    report["readStatus"] = Int(readStatus)
    report["authenticatedReadVerified"] = interactive && readStatus == errSecSuccess
    report["storageReadVerified"] = readStatus == errSecSuccess
    report["unauthenticatedReadDenied"] = dataProtection && !interactive && readStatus != errSecSuccess
    item = nil

    if !dataProtection, let readerOffset = CommandLine.arguments.firstIndex(of: "--reader"),
      CommandLine.arguments.count > readerOffset + 1 {
      let process = Process()
      process.executableURL = URL(fileURLWithPath: CommandLine.arguments[readerOffset + 1])
      process.arguments = ["--foreign-read", service]
      let pipe = Pipe()
      process.standardOutput = pipe
      do {
        try process.run()
        process.waitUntilExit()
        let output = pipe.fileHandleForReading.readDataToEndOfFile()
        if let text = String(data: output, encoding: .utf8),
          let status = Int(text.trimmingCharacters(in: .whitespacesAndNewlines)) {
          report["foreignReadStatus"] = status
          report["foreignReadDenied"] = status != errSecSuccess
        }
      } catch {
        report["foreignReaderFailed"] = true
      }
    }

    if interactive && dataProtection && readStatus == errSecSuccess {
      report["sameContextReadStatus"] = Int(SecItemCopyMatching(read as CFDictionary, nil))
      let fresh = LAContext()
      fresh.touchIDAuthenticationAllowableReuseDuration = 0
      read[kSecUseAuthenticationContext as String] = fresh
      report["freshContextReadStatus"] = Int(SecItemCopyMatching(read as CFDictionary, nil))
      fresh.invalidate()
    }
    context.invalidate()
    report["cleanupStatus"] = Int(SecItemDelete(identity as CFDictionary))
    emit(report)
  }

  static func emit(_ report: [String: Any]) {
    if let data = try? JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]),
      let text = String(data: data, encoding: .utf8) {
      print(text)
    }
  }
}
