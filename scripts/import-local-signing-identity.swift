import Foundation
import Security

// Passphrase arrives over stdin, never command-line arguments. Only the newly
// imported signing key gets an ACL, restricted to Apple's codesign executable.
@main
struct ImportLocalSigningIdentity {
    struct Request: Decodable { let path: String; let password: String }
    static func main() {
        SecKeychainSetUserInteractionAllowed(false)
        do {
            let request = try JSONDecoder().decode(Request.self, from: FileHandle.standardInput.readDataToEndOfFile())
            let file = URL(fileURLWithPath: request.path).standardizedFileURL
            guard file.lastPathComponent == "identity.p12" || file.lastPathComponent == "test-identity.p12" else { exit(4) }
            var keychain: SecKeychain?
            if CommandLine.arguments.count == 2 {
                let path = URL(fileURLWithPath: CommandLine.arguments[1]).standardizedFileURL.path
                guard path.contains("/.artifacts/keychain-upgrade-probe/"),
                    URL(fileURLWithPath: path).lastPathComponent.hasPrefix("topotp-test-") else { exit(4) }
                guard SecKeychainOpen(path, &keychain) == errSecSuccess else { exit(4) }
            } else {
                guard CommandLine.arguments.count == 1,
                    SecKeychainCopyDefault(&keychain) == errSecSuccess else { exit(4) }
            }
            guard let keychain else { exit(4) }
            var application: SecTrustedApplication?
            guard SecTrustedApplicationCreateFromPath("/usr/bin/codesign", &application) == errSecSuccess,
                let application else { exit(4) }
            var access: SecAccess?
            guard SecAccessCreate("TopOTP Local Signing" as CFString, [application] as CFArray, &access) == errSecSuccess,
                let access else { exit(4) }
            let options: [String: Any] = [
                kSecImportExportPassphrase as String: request.password,
                kSecImportExportKeychain as String: keychain,
                kSecImportExportAccess as String: access,
            ]
            var imported: CFArray?
            let status = SecPKCS12Import(try Data(contentsOf: file) as CFData, options as CFDictionary, &imported)
            print("{\"status\":\(status)}")
            if status != errSecSuccess { exit(2) }
        } catch { fputs("Não foi possível importar a identidade local.\n", stderr); exit(2) }
    }
}
