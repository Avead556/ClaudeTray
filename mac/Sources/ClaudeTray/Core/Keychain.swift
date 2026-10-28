import Foundation

enum Keychain {
    static func read(service: String, account: String? = nil) -> String? {
        var args = ["find-generic-password", "-s", service, "-w"]
        if let account { args += ["-a", account] }
        guard let result = try? run(args), result.status == 0 else { return nil }
        let value = result.stdout.trimmingCharacters(in: .newlines)
        return value.isEmpty ? nil : value
    }

    static func account(service: String) -> String? {
        guard let result = try? run(["find-generic-password", "-s", service]), result.status == 0 else { return nil }
        for line in result.stdout.split(separator: "\n") {
            guard let range = line.range(of: "\"acct\"<blob>=\"") else { continue }
            return String(line[range.upperBound...].dropLast())
        }
        return nil
    }

    static func write(service: String, account: String, secret: String) throws {
        let hex = Data(secret.utf8).map { String(format: "%02x", $0) }.joined()
        let command = "add-generic-password -U -a \(quote(account)) -s \(quote(service)) -X \(hex)\n"
        let result = command.utf8.count < 4000
            ? try run(["-i"], stdin: command)
            : try run(["add-generic-password", "-U", "-a", account, "-s", service, "-X", hex])
        guard read(service: service, account: account) == secret else {
            throw AppError(String(localized: "Couldn’t write “\(service)” to the Keychain.") + " " + result.stderr)
        }
    }

    static func delete(service: String, account: String) {
        _ = try? run(["delete-generic-password", "-s", service, "-a", account])
    }

    private static func quote(_ s: String) -> String {
        "\"" + s.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"") + "\""
    }

    private struct Result { let status: Int32; let stdout: String; let stderr: String }

    private static func run(_ args: [String], stdin: String? = nil) throws -> Result {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/security")
        process.arguments = args
        let out = Pipe(), err = Pipe(), input = Pipe()
        process.standardOutput = out
        process.standardError = err
        if stdin != nil { process.standardInput = input }
        try process.run()
        if let stdin {
            input.fileHandleForWriting.write(Data(stdin.utf8))
            try? input.fileHandleForWriting.close()
        }
        let outData = out.fileHandleForReading.readDataToEndOfFile()
        let errData = err.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return Result(status: process.terminationStatus,
                      stdout: String(decoding: outData, as: UTF8.self),
                      stderr: String(decoding: errData, as: UTF8.self))
    }
}
