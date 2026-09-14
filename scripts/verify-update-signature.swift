// Verify the actual update bytes with the public key embedded in AppDock.
import CryptoKit
import Foundation
if CommandLine.arguments.count != 4 {
    fputs("Usage: verify-update-signature public-key signature archive\n", stderr)
    exit(2)
}
do {
    guard let publicBytes = Data(base64Encoded: CommandLine.arguments[1]),
          let signature = Data(base64Encoded: CommandLine.arguments[2]) else {
        throw NSError(domain: "AppDock", code: 1)
    }
    let publicKey = try Curve25519.Signing.PublicKey(rawRepresentation: publicBytes)
    let archive = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[3]), options: .mappedIfSafe)
    guard publicKey.isValidSignature(signature, for: archive) else {
        fputs("Update signature does not match the embedded public key or archive bytes.\n", stderr)
        exit(1)
    }
    print("Update signature verified with the embedded public key.")
} catch {
    fputs("Invalid update signature input.\n", stderr)
    exit(1)
}
