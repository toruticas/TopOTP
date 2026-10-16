// Only random synthetic namespaces in the default login Keychain are allowed.
import Foundation
import Security

let args = CommandLine.arguments
let prefix = "com.rafaelmariano.topotp.upgrade-probe."
guard args.count == 3, args[2].hasPrefix(prefix),
  UUID(uuidString: String(args[2].dropFirst(prefix.count))) != nil
else { exit(4) }
SecKeychainSetUserInteractionAllowed(false)
let payload = Data("TopOTP public direct upgrade fixture".utf8)
let identity: [String: Any] = [
  kSecClass as String: kSecClassGenericPassword,
  kSecAttrService as String: args[2], kSecAttrAccount as String: "synthetic-control",
  kSecUseDataProtectionKeychain as String: false, kSecAttrSynchronizable as String: false,
]
var query = identity
var status: OSStatus
var matched = false
if args[1] == "add" {
  query[kSecValueData as String] = payload
  status = SecItemAdd(query as CFDictionary, nil)
} else if args[1] == "read" {
  query[kSecReturnData as String] = true
  var value: CFTypeRef?
  status = SecItemCopyMatching(query as CFDictionary, &value)
  matched = (value as? Data) == payload
} else if args[1] == "delete" {
  status = SecItemDelete(query as CFDictionary)
} else {
  exit(4)
}
#if PROBE_V2
  let variant = 2
#else
  let variant = 1
#endif
print(
  String(
    decoding: try JSONSerialization.data(withJSONObject: [
      "status": Int(status), "matched": matched, "variant": variant,
    ]), as: UTF8.self))
