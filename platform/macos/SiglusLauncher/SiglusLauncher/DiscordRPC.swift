import Foundation
import AppKit

/// 最強の Discord Rich Presence (RPC) マネージャー
/// - 即時同期接続 & 確実なアクティビティ送信
/// - 自動再接続常駐
/// - ゲームアイコンの Catbox.moe への非同期自動アップロード & キャッシュ
private func logRPC(_ msg: String) {
    let line = "[DiscordRPC] " + msg + "\n"
    print(line, terminator: "")
    fflush(stdout)
    if let data = line.data(using: .utf8) {
        let logUrl = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first!
            .appendingPathComponent("Logs/SiglusLauncher/console.log")
        if let handle = try? FileHandle(forWritingTo: logUrl) {
            handle.seekToEndOfFile()
            handle.write(data)
            try? handle.close()
        }
    }
}

final class DiscordRPC: @unchecked Sendable {
    static let shared = DiscordRPC()

    private let defaultClientId = "1559041354905034852"
    private var clientId: String {
        ProcessInfo.processInfo.environment["SIGLUS_DISCORD_CLIENT_ID"] ?? defaultClientId
    }

    private let lock = NSLock()
    private var socketFd: Int32 = -1
    private var isConnected = false
    private var shouldRun = true
    private var currentActivity: ActivityPayload? = nil
    private let launcherStartTime = Int64(Date().timeIntervalSince1970)

    private var cacheFileURL: URL {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SiglusLauncher", isDirectory: true)
        try? FileManager.default.createDirectory(at: appSupport, withIntermediateDirectories: true)
        return appSupport.appendingPathComponent("catbox_cache.json")
    }

    private var catboxCache: [String: String] = [:]

    struct ActivityPayload: Equatable {
        var state: String
        var details: String
        var startTimestamp: Int64
        var largeImage: String?
        var largeText: String?
        var smallImage: String?
        var smallText: String?
        var buttonLabel: String?
        var buttonURL: String?
    }

    private init() {
        loadCatboxCache()
        // 即座に接続を試みる
        _ = connectToDiscord()
        startWorker()
    }

    deinit {
        shouldRun = false
        disconnect()
    }

    // -------------------------------------------------------------------------
    // パブリック API
    // -------------------------------------------------------------------------

    /// ランチャー閲覧中（待機状態）のステータスを設定
    func setIdle(gameCount: Int) {
        let act = ActivityPayload(
            state: "ライブラリ閲覧中",
            details: "ゲームを選択中… (登録: \(gameCount)本)",
            startTimestamp: launcherStartTime,
            largeImage: "https://files.catbox.moe/3umu8k.png",
            largeText: "Siglus ランチャー",
            smallImage: nil,
            smallText: "siglus_rs on macOS",
            buttonLabel: "GitHub リポジトリ",
            buttonURL: "https://github.com/mirinnano/siglus_rs"
        )
        updateActivity(act)
    }

    /// ゲームプレイ中（アクティブ状態）のステータスを設定
    func setPlaying(game: GameEntry) {
        let startTime = Int64(Date().timeIntervalSince1970)
        let initialImage = catboxCache[game.rootPath] ?? "https://files.catbox.moe/22piqb.png"

        // ユーザー指定: 「状態をゲームタイトルにしたい」 -> state: game.title
        let act = ActivityPayload(
            state: game.title,
            details: "\(game.engineName) でプレイ中",
            startTimestamp: startTime,
            largeImage: initialImage,
            largeText: game.title,
            smallImage: nil,
            smallText: "siglus_rs on macOS",
            buttonLabel: "GitHub リポジトリ",
            buttonURL: "https://github.com/mirinnano/siglus_rs"
        )
        // 即座に初期アクティビティを送信（画像アップロード完了を待たずに即時反映！）
        updateActivity(act)

        // バックグラウンドでアイコンを Catbox にアップロードして、完了後に URL を差し替え
        uploadGameIconIfNeeded(game: game) { [weak self] uploadedUrl in
            guard let self = self, let url = uploadedUrl else { return }
            self.lock.lock()
            if var current = self.currentActivity, current.state == game.title {
                current.largeImage = url
                self.currentActivity = current
                self.lock.unlock()
                self.sendActivity(current)
            } else {
                self.lock.unlock()
            }
        }
    }

    /// プレゼンスをクリア
    func clearActivity() {
        lock.lock()
        currentActivity = nil
        lock.unlock()
        if isConnected {
            sendClearActivity()
        }
    }

    // -------------------------------------------------------------------------
    // 内部ソケット通信 & 送信ワーカー
    // -------------------------------------------------------------------------

    private func updateActivity(_ activity: ActivityPayload) {
        lock.lock()
        currentActivity = activity
        let connected = isConnected
        lock.unlock()

        if !connected {
            _ = connectToDiscord()
        }

        lock.lock()
        let nowConnected = isConnected
        lock.unlock()

        if nowConnected {
            sendActivity(activity)
        }
    }

    private func startWorker() {
        DispatchQueue.global(qos: .default).async { [weak self] in
            while let self = self, self.shouldRun {
                self.ensureConnection()
                Thread.sleep(forTimeInterval: 5.0)
            }
        }
    }

    private func ensureConnection() {
        lock.lock()
        let connected = isConnected
        lock.unlock()

        if !connected {
            if connectToDiscord() {
                lock.lock()
                let act = currentActivity
                lock.unlock()
                if let act = act {
                    sendActivity(act)
                }
            }
        }
    }

    private func findDiscordSocketPath() -> String? {
        let envs = [
            NSTemporaryDirectory(),
            ProcessInfo.processInfo.environment["TMPDIR"],
            "/tmp",
            ProcessInfo.processInfo.environment["XDG_RUNTIME_DIR"]
        ].compactMap { $0 }

        let fm = FileManager.default
        for dir in envs {
            for i in 0..<10 {
                let path = (dir as NSString).appendingPathComponent("discord-ipc-\(i)")
                if fm.fileExists(atPath: path) {
                    return path
                }
            }
        }
        return nil
    }

    @discardableResult
    private func connectToDiscord() -> Bool {
        lock.lock()
        if isConnected && socketFd >= 0 {
            lock.unlock()
            return true
        }
        lock.unlock()

        guard let sockPath = findDiscordSocketPath() else {
            logRPC("Discord IPC socket not found yet.")
            return false
        }
        logRPC("Found socket path: " + sockPath)

        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { return false }

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        addr.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)

        let maxLen = MemoryLayout.size(ofValue: addr.sun_path)
        sockPath.withCString { ptr in
            _ = withUnsafeMutablePointer(to: &addr.sun_path) { pathPtr in
                strncpy(UnsafeMutableRawPointer(pathPtr).assumingMemoryBound(to: CChar.self), ptr, maxLen - 1)
            }
        }

        let connectResult = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { saPtr in
                connect(fd, saPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }

        guard connectResult == 0 else {
            logRPC("connect() failed with errno: \(errno)")
            close(fd)
            return false
        }
        logRPC("connect() succeeded! Sending handshake...")

        // Handshake 送信 (Opcode 0)
        let handshakeObj: [String: Any] = [
            "v": 1,
            "client_id": clientId
        ]
        guard let handshakeData = try? JSONSerialization.data(withJSONObject: handshakeObj) else {
            close(fd)
            return false
        }

        if !writePacket(fd: fd, opcode: 0, data: handshakeData) {
            logRPC("writePacket(handshake) failed!")
            close(fd)
            return false
        }
        logRPC("Handshake sent! Waiting for response...")

        // Handshake 応答を受信
        if let (op, body) = readPacket(fd: fd) {
            logRPC("Received handshake opcode: \(op) len: \(body.count)")
            if op == 1 {
                lock.lock()
                self.socketFd = fd
                self.isConnected = true
                lock.unlock()
                logRPC("Connected and Handshake READY successfully! (socket: \(sockPath))")
                return true
            } else {
                logRPC("Unexpected handshake opcode: \(op)")
                close(fd)
                return false
            }
        } else {
            logRPC("readPacket(handshake) failed!")
            close(fd)
            return false
        }
    }

    private func disconnect() {
        lock.lock()
        if socketFd >= 0 {
            close(socketFd)
            socketFd = -1
        }
        isConnected = false
        lock.unlock()
    }

    private func writePacket(fd: Int32, opcode: UInt32, data: Data) -> Bool {
        var header = [opcode.littleEndian, UInt32(data.count).littleEndian]
        let headerData = Data(bytes: &header, count: 8)
        var fullData = headerData
        fullData.append(data)

        var totalSent = 0
        let count = fullData.count
        return fullData.withUnsafeBytes { rawPtr -> Bool in
            guard let base = rawPtr.baseAddress else { return false }
            while totalSent < count {
                let sent = write(fd, base.advanced(by: totalSent), count - totalSent)
                if sent <= 0 { return false }
                totalSent += sent
            }
            return true
        }
    }

    private func readPacket(fd: Int32) -> (UInt32, Data)? {
        var header = [UInt32](repeating: 0, count: 2)
        let r = read(fd, &header, 8)
        guard r == 8 else { return nil }

        let opcode = UInt32(littleEndian: header[0])
        let length = Int(UInt32(littleEndian: header[1]))
        guard length >= 0 && length < 65536 else { return nil }

        var body = Data(count: length)
        var totalRead = 0
        while totalRead < length {
            let chunk = body.withUnsafeMutableBytes { ptr -> Int in
                guard let base = ptr.baseAddress else { return -1 }
                return read(fd, base.advanced(by: totalRead), length - totalRead)
            }
            if chunk <= 0 { return nil }
            totalRead += chunk
        }
        return (opcode, body)
    }

    private func sendActivity(_ act: ActivityPayload) {
        lock.lock()
        let fd = socketFd
        let connected = isConnected
        lock.unlock()

        guard connected, fd >= 0 else { return }

        var activityDict: [String: Any] = [
            "state": act.state,
            "details": act.details,
            "timestamps": [
                "start": act.startTimestamp
            ]
        ]

        var assetsDict: [String: Any] = [:]
        if let lImg = act.largeImage { assetsDict["large_image"] = lImg }
        if let lTxt = act.largeText { assetsDict["large_text"] = lTxt }
        if let sImg = act.smallImage { assetsDict["small_image"] = sImg }
        if let sTxt = act.smallText { assetsDict["small_text"] = sTxt }
        if !assetsDict.isEmpty {
            activityDict["assets"] = assetsDict
        }

        if let bLabel = act.buttonLabel, let bUrl = act.buttonURL {
            activityDict["buttons"] = [
                ["label": bLabel, "url": bUrl]
            ]
        }

        let payload: [String: Any] = [
            "cmd": "SET_ACTIVITY",
            "args": [
                "pid": ProcessInfo.processInfo.processIdentifier,
                "activity": activityDict
            ],
            "nonce": UUID().uuidString
        ]

        guard let data = try? JSONSerialization.data(withJSONObject: payload) else { return }
        if writePacket(fd: fd, opcode: 1, data: data) {
            logRPC("Activity successfully pushed: \(act.state) - \(act.details)")
            // 応答フレームを消費
            _ = readPacket(fd: fd)
        } else {
            logRPC("Failed to send activity, disconnecting...")
            disconnect()
        }
    }

    private func sendClearActivity() {
        lock.lock()
        let fd = socketFd
        let connected = isConnected
        lock.unlock()

        guard connected, fd >= 0 else { return }

        let payload: [String: Any] = [
            "cmd": "SET_ACTIVITY",
            "args": [
                "pid": ProcessInfo.processInfo.processIdentifier,
                "activity": NSNull()
            ],
            "nonce": UUID().uuidString
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: payload) else { return }
        _ = writePacket(fd: fd, opcode: 1, data: data)
    }

    // -------------------------------------------------------------------------
    // Catbox.moe 画像アップロード & キャッシュ管理
    // -------------------------------------------------------------------------

    private func loadCatboxCache() {
        guard FileManager.default.fileExists(atPath: cacheFileURL.path),
              let data = try? Data(contentsOf: cacheFileURL),
              let dict = try? JSONDecoder().decode([String: String].self, from: data) else {
            return
        }
        self.catboxCache = dict
    }

    private func saveCatboxCache() {
        if let data = try? JSONEncoder().encode(catboxCache) {
            try? data.write(to: cacheFileURL, options: .atomic)
        }
    }

    private func uploadGameIconIfNeeded(game: GameEntry, completion: @escaping @Sendable (String?) -> Void) {
        if let cached = catboxCache[game.rootPath] {
            completion(cached)
            return
        }

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            let pngData = self.resolveGameIconPNG(game: game)

            guard let data = pngData else {
                completion(nil)
                return
            }

            self.uploadToCatbox(imageData: data, filename: "\(game.id).png") { url in
                if let url = url {
                    DispatchQueue.main.async {
                        self.catboxCache[game.rootPath] = url
                        self.saveCatboxCache()
                        logRPC("Catbox uploaded: \(url) for \(game.title)")
                    }
                }
                completion(url)
            }
        }
    }

    private func resolveGameIconPNG(game: GameEntry) -> Data? {
        let fm = FileManager.default
        let rootURL = URL(fileURLWithPath: game.rootPath)

        var candidatePaths: [String] = []
        if let cp = game.coverPath, fm.fileExists(atPath: cp) {
            candidatePaths.append(cp)
        }

        let commonIcons = ["icon.png", "icon.ico", "Icon.png", "Icon.ico", "app.ico", "LB_EX.ico"]
        for name in commonIcons {
            let p = rootURL.appendingPathComponent(name).path
            if fm.fileExists(atPath: p) {
                candidatePaths.append(p)
            }
        }

        for path in candidatePaths {
            if let image = NSImage(contentsOfFile: path) {
                if let tiff = image.tiffRepresentation,
                   let rep = NSBitmapImageRep(data: tiff),
                   let png = rep.representation(using: .png, properties: [:]) {
                    return png
                }
            }
        }
        return nil
    }

    private func uploadToCatbox(imageData: Data, filename: String, completion: @escaping @Sendable (String?) -> Void) {
        guard let apiURL = URL(string: "https://catbox.moe/user/api.php") else {
            completion(nil)
            return
        }

        let q = String(Character(UnicodeScalar(34)))
        let crlf = String(bytes: [13, 10], encoding: .utf8)!
        let boundary = "----WebKitFormBoundary" + UUID().uuidString.replacingOccurrences(of: "-", with: "")

        var request = URLRequest(url: apiURL)
        request.httpMethod = "POST"
        request.setValue("multipart/form-data; boundary=" + boundary, forHTTPHeaderField: "Content-Type")
        request.setValue("siglus_rs-rpc/1.0", forHTTPHeaderField: "User-Agent")
        request.timeoutInterval = 15

        var body = Data()

        // reqtype
        let part1 = "--" + boundary + crlf + "Content-Disposition: form-data; name=" + q + "reqtype" + q + crlf + crlf + "fileupload" + crlf
        if let d1 = part1.data(using: .utf8) { body.append(d1) }

        // fileToUpload
        let part2 = "--" + boundary + crlf + "Content-Disposition: form-data; name=" + q + "fileToUpload" + q + "; filename=" + q + filename + q + crlf + "Content-Type: image/png" + crlf + crlf
        if let d2 = part2.data(using: .utf8) { body.append(d2) }
        body.append(imageData)

        let closing = crlf + "--" + boundary + "--" + crlf
        if let dc = closing.data(using: .utf8) { body.append(dc) }

        request.httpBody = body

        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            guard error == nil, let data = data, let str = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
                  str.hasPrefix("http") else {
                completion(nil)
                return
            }
            completion(str)
        }
        task.resume()
    }
}
