import Darwin
// Version 1 is immutable once deployed. A changed helper requires explicit migration.
// Private inherited socket; secrets never use stdout, arguments, files or logs.
import Foundation
import Security

let certificate = "@@SIGNING_CERTIFICATE_SHA1@@"
let stream = FileHandle.standardInput
func readExactly(_ length: Int) throws -> Data? {
  var data = Data()
  while data.count < length {
    guard let part = try stream.read(upToCount: length - data.count), !part.isEmpty else {
      return nil
    }
    data.append(part)
  }
  return data
}
func writeResponse(_ status: OSStatus, _ payload: Data = Data()) throws {
  var code = Int32(status).bigEndian
  var count = UInt32(payload.count).bigEndian
  var data = Data(bytes: &code, count: 4)
  data.append(Data(bytes: &count, count: 4))
  data.append(payload)
  defer { data.resetBytes(in: 0..<data.count) }
  try stream.write(contentsOf: data)
}
var callerCode: SecCode?
var callerRequirement: SecRequirement?
func callerStillValid() -> Bool {
  guard let code = callerCode, let requirement = callerRequirement else { return false }
  return SecCodeCheckValidity(code, SecCSFlags(rawValue: kSecCSStrictValidate), requirement) == 0
}
func caller() -> (OSStatus, String?) {
  var token = audit_token_t()
  var size = socklen_t(MemoryLayout.size(ofValue: token))
  let result = withUnsafeMutablePointer(to: &token) {
    getsockopt(STDIN_FILENO, SOL_LOCAL, LOCAL_PEERTOKEN, $0, &size)
  }
  guard result == 0, size == MemoryLayout.size(ofValue: token) else { return (errSecParam, nil) }
  let audit = Data(bytes: &token, count: Int(size))
  var code: SecCode?
  var status = SecCodeCopyGuestWithAttributes(
    nil, [kSecGuestAttributeAudit as String: audit] as CFDictionary, [], &code)
  guard status == 0, let code else { return (status, nil) }
  var staticCode: SecStaticCode?
  status = SecCodeCopyStaticCode(code, [], &staticCode)
  guard status == 0, let staticCode else { return (status, nil) }
  var info: CFDictionary?
  status = SecCodeCopySigningInformation(
    staticCode, SecCSFlags(rawValue: kSecCSSigningInformation), &info)
  guard status == 0, let fields = info as? [String: Any],
    let identifier = fields[kSecCodeInfoIdentifier as String] as? String,
    identifier == "com.rafaelmariano.topotp"
      || (identifier.hasPrefix("com.rafaelmariano.topotp.") && identifier.count < 180
        && identifier.unicodeScalars.allSatisfy({
          CharacterSet(
            charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-_"
          ).contains($0)
        }))
  else { return (errSecCSReqFailed, nil) }
  var requirement: SecRequirement?
  status = SecRequirementCreateWithString(
    ("identifier \"" + identifier + "\" and certificate leaf = H\"" + certificate + "\"")
      as CFString, [], &requirement)
  guard status == 0, let requirement else { return (status, nil) }
  status = SecCodeCheckValidity(code, SecCSFlags(rawValue: kSecCSStrictValidate), requirement)
  if status == 0 {
    callerCode = code
    callerRequirement = requirement
  }
  return (status, status == 0 ? identifier : nil)
}
func serve() throws {
  SecKeychainSetUserInteractionAllowed(false)
  try stream.write(contentsOf: Data("TOPOTP01".utf8))
  let (status, identifier) = caller()
  guard status == 0, let identifier else {
    try? writeResponse(status == 0 ? errSecCSReqFailed : status)
    exit(1)
  }
  try writeResponse(0)
  let service = identifier + ".broker.v1"
  // Peer code is revalidated before every operation, including after exec/exit.
  while let header = try readExactly(21) {
    let operation = header[0]
    let idData = header.subdata(in: 1..<17)
    let uuid = idData.withUnsafeBytes { UUID(uuid: $0.load(as: uuid_t.self)) }.uuidString
      .lowercased()
    let length = header[17..<21].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    guard (1...4).contains(operation), length <= 32768,
      (operation == 2 || operation == 3) ? length > 0 : length == 0
    else {
      try writeResponse(errSecParam)
      exit(2)
    }
    guard var payload = try readExactly(Int(length)) else { exit(2) }
    defer { payload.resetBytes(in: 0..<payload.count) }
    guard callerStillValid() else {
      try writeResponse(errSecCSReqFailed)
      exit(1)
    }
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
      kSecAttrAccount as String: uuid, kSecUseDataProtectionKeychain as String: false,
      kSecAttrSynchronizable as String: false,
    ]
    if operation == 1 {
      var read = query
      read[kSecReturnData as String] = true
      var value: CFTypeRef?
      let status = SecItemCopyMatching(read as CFDictionary, &value)
      var bytes = value as? Data ?? Data()
      defer { bytes.resetBytes(in: 0..<bytes.count) }
      guard bytes.count <= 32768 else {
        try writeResponse(errSecParam)
        exit(2)
      }
      guard callerStillValid() else {
        try writeResponse(errSecCSReqFailed)
        exit(1)
      }
      try writeResponse(status, bytes)
    } else if operation == 2 {
      var create = query
      create[kSecValueData as String] = payload
      try writeResponse(SecItemAdd(create as CFDictionary, nil))
    } else if operation == 3 {
      try writeResponse(
        SecItemUpdate(query as CFDictionary, [kSecValueData as String: payload] as CFDictionary))
    } else {
      try writeResponse(SecItemDelete(query as CFDictionary))
    }
  }

}
do { try serve() } catch { exit(1) }
