import Foundation
import Security

// Only disposable keychains under the probe directory are accepted. This tool
// cannot query the login Keychain and never enables Security's password UI.
@main
struct KeychainUpdateProbe {
    static func main() {
        SecKeychainSetUserInteractionAllowed(false)
        let args = CommandLine.arguments
        guard args.count == 4,
            let password = ProcessInfo.processInfo.environment["TOPOTP_SYNTHETIC_PASSWORD"]
        else { exit(4) }
        let mode = args[1]
        let path = URL(fileURLWithPath: args[2]).standardizedFileURL.path
        let service = args[3]
        guard path.contains("/.artifacts/keychain-upgrade-probe/"),
            URL(fileURLWithPath: path).lastPathComponent.hasPrefix("topotp-test-"),
            service.hasPrefix("com.rafaelmariano.topotp.upgrade-probe.")
        else { exit(4) }
        var chain: SecKeychain?
        var status: OSStatus
        if mode == "create-chain" {
            var previous: CFArray?
            guard SecKeychainCopySearchList(&previous) == errSecSuccess, let previous else { exit(4) }
            defer { SecKeychainSetSearchList(previous) }
            status = password.withCString {
                SecKeychainCreate(path, UInt32(password.utf8.count), $0, false, nil, &chain)
            }
        } else {
            status = SecKeychainOpen(path, &chain)
            if status == errSecSuccess, let chain {
                status = password.withCString {
                    SecKeychainUnlock(chain, UInt32(password.utf8.count), $0, true)
                }
            }
        }
        guard status == errSecSuccess, let chain else { emit(status, false); return }
        if mode == "create-chain" { emit(status, true); return }
        if mode == "delete-chain" { emit(SecKeychainDelete(chain), true); return }
        let payload = Data("TopOTP public synthetic upgrade fixture".utf8)
        let identity: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: "synthetic-control",
            kSecUseDataProtectionKeychain as String: false,
            kSecAttrSynchronizable as String: false,
        ]
        var query = identity
        query[kSecMatchSearchList as String] = [chain]
        if mode == "add" {
            var create = identity
            create[kSecUseKeychain as String] = chain
            create[kSecValueData as String] = payload
            status = SecItemAdd(create as CFDictionary, nil)
            emit(status, status == errSecSuccess)
        } else if mode == "read" {
            query[kSecReturnData as String] = true
            var value: CFTypeRef?
            status = SecItemCopyMatching(query as CFDictionary, &value)
            emit(status, (value as? Data) == payload)
        } else if mode == "update" {
            status = SecItemUpdate(query as CFDictionary, [kSecValueData as String: payload] as CFDictionary)
            emit(status, status == errSecSuccess)
        } else if mode == "delete" {
            emit(SecItemDelete(query as CFDictionary), true)
        } else { exit(4) }
    }
    static func emit(_ status: OSStatus, _ matched: Bool) {
        #if PROBE_V2
        let variant = 2
        #else
        let variant = 1
        #endif
        let data = try! JSONSerialization.data(withJSONObject: [
            "status": Int(status), "syntheticPayloadMatched": matched,
            "variant": variant, "keychainUIAllowed": false,
        ], options: [.sortedKeys])
        print(String(data: data, encoding: .utf8)!)
    }
}
