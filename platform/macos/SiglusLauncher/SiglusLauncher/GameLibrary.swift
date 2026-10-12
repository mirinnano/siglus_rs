import Foundation
import AppKit
import SwiftUI

// -----------------------------
// Rust launcher API (game_launcher)
// -----------------------------
@_silgen_name("game_scan_json")
private func game_scan_json(_ path: UnsafePointer<CChar>, _ depth: Int32, _ coverCacheDir: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("game_probe_json")
private func game_probe_json(_ root: UnsafePointer<CChar>, _ nls: UnsafePointer<CChar>?, _ coverCacheDir: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("game_string_free")
private func game_string_free(_ ptr: UnsafeMutablePointer<CChar>?) -> Void

private func takeString(_ ptr: UnsafeMutablePointer<CChar>?) -> String? {
    guard let ptr else { return nil }
    let out = String(cString: ptr)
    game_string_free(ptr)
    return out
}

// -----------------------------
// Model
// -----------------------------
struct NlsOption: Codable, Equatable, Hashable {
    let id: String
    let label: String

    var japaneseLabel: String {
        switch id.lowercased() {
        case "sjis", "shift-jis", "shift_jis", "cp932":
            return "Shift-JIS (日本語)"
        case "gbk", "gb2312", "cp936":
            return "GBK (簡体字中国語)"
        case "big5", "cp950":
            return "Big5 (繁体字中国語)"
        case "utf8", "utf-8":
            return "UTF-8"
        case "western", "cp1252":
            return "Windows-1252 (欧米・英語)"
        case "korean", "cp949", "euc-kr":
            return "CP949 (韓国語)"
        case "auto":
            return "自動判別 (RLdev記録)"
        default:
            return label
        }
    }
}

/// One result of the Rust probe (see GameInfo::to_json).
struct ProbedGame: Decodable {
    let id: String?
    let root: String
    let engine: String
    let engine_name: String?
    let supported: Bool
    let unsupported_reason: String?
    let title: String
    let cover: String?
    let cover_kind: String?
    let nls: String?
    let nls_options: [NlsOption]?
}

struct GameEntry: Identifiable, Codable, Equatable {
    let id: String
    var title: String
    var rootPath: String
    var addedAtUnix: Int64
    var lastPlayedAtUnix: Int64?
    var coverPath: String?
    /// "siglus", "reallive", "avg32" or "uk2".
    var engine: String
    var engineName: String
    /// "image" (full-bleed picture) or "icon" (generated icon card).
    var coverKind: String?
    /// Text encoding for RealLive/AVG32/UK2 (nil for SiglusEngine).
    var nls: String?
    var nlsOptions: [NlsOption]

    init(id: String, title: String, rootPath: String, addedAtUnix: Int64,
         lastPlayedAtUnix: Int64? = nil, coverPath: String? = nil,
         engine: String = "siglus", engineName: String = "SiglusEngine",
         coverKind: String? = nil, nls: String? = nil, nlsOptions: [NlsOption] = []) {
        self.id = id
        self.title = title
        self.rootPath = rootPath
        self.addedAtUnix = addedAtUnix
        self.lastPlayedAtUnix = lastPlayedAtUnix
        self.coverPath = coverPath
        self.engine = engine
        self.engineName = engineName
        self.coverKind = coverKind
        self.nls = nls
        self.nlsOptions = nlsOptions
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        rootPath = try c.decode(String.self, forKey: .rootPath)
        addedAtUnix = try c.decode(Int64.self, forKey: .addedAtUnix)
        lastPlayedAtUnix = try c.decodeIfPresent(Int64.self, forKey: .lastPlayedAtUnix)
        coverPath = try c.decodeIfPresent(String.self, forKey: .coverPath)
        engine = try c.decodeIfPresent(String.self, forKey: .engine) ?? "siglus"
        engineName = try c.decodeIfPresent(String.self, forKey: .engineName) ?? "SiglusEngine"
        coverKind = try c.decodeIfPresent(String.self, forKey: .coverKind)
        nls = try c.decodeIfPresent(String.self, forKey: .nls)
        nlsOptions = try c.decodeIfPresent([NlsOption].self, forKey: .nlsOptions) ?? []
    }

    var nlsLabel: String? {
        guard let nls else { return nil }
        return nlsOptions.first(where: { $0.id == nls })?.japaneseLabel ?? nls
    }

    var keyTomlURL: URL {
        URL(fileURLWithPath: rootPath).appendingPathComponent("key.toml")
    }

    var hasKeyToml: Bool {
        FileManager.default.fileExists(atPath: keyTomlURL.path)
    }

    var lastPlayedString: String {
        guard let unix = lastPlayedAtUnix else { return "未プレイ" }
        let date = Date(timeIntervalSince1970: TimeInterval(unix))
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "ja_JP")
        formatter.dateStyle = .short
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    var addedDateString: String {
        let date = Date(timeIntervalSince1970: TimeInterval(addedAtUnix))
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "ja_JP")
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}

/// Summary shown after an import.
struct ImportReport: Identifiable {
    let id = UUID()
    let imported: [String]
    let updated: [String]
    let unsupported: [(String, String)]
    let empty: [String]

    var message: String {
        var lines: [String] = []
        if !imported.isEmpty {
            lines.append("【追加完了】\(imported.count) 本のゲームを追加しました：\n・" + imported.joined(separator: "\n・"))
        }
        if !updated.isEmpty {
            lines.append("【更新完了】ライブラリ内の既存ゲーム \(updated.count) 本を再スキャンして更新しました。")
        }
        if !unsupported.isEmpty {
            var unsList = "【非対応】以下のゲームは現在サポートされていません：\n"
            for (title, reason) in unsupported {
                unsList += "・\(title) (理由: \(reason))\n"
            }
            lines.append(unsList)
        }
        if !empty.isEmpty {
            var empList = "【ゲーム未検出】ゲームファイルが見つかりませんでした：\n"
            for path in empty {
                empList += "・\(path)\n"
            }
            empList += "※ Gameexe.ini, Scene.pck, SEEN.TXT, UK2.CFG などが含まれるゲームのルートフォルダを選択してください。"
            lines.append(empList)
        }
        return lines.joined(separator: "\n\n")
    }

    var title: String {
        if imported.isEmpty && updated.isEmpty {
            return "ゲームが追加されませんでした"
        }
        return unsupported.isEmpty && empty.isEmpty ? "インポート完了" : "インポート結果 (確認事項あり)"
    }
}

enum GameSortOption: String, CaseIterable, Identifiable {
    case lastPlayed = "最近プレイ順"
    case title = "タイトル順"
    case added = "追加日順"

    var id: String { rawValue }
}

@MainActor
final class GameLibrary: ObservableObject {
    @Published var games: [GameEntry] = []
    @Published var showError: Bool = false
    @Published var errorMessage: String = ""
    @Published var importReport: ImportReport? = nil
    @Published var isImporting: Bool = false
    @Published var searchText: String = ""
    @Published var selectedEngineFilter: String = "all"
    @Published var sortOption: GameSortOption = .lastPlayed
    @Published var selectedGameForDetail: GameEntry? = nil

    var onLaunchRequest: ((GameEntry) -> Void)? = nil

    private let fm = FileManager.default

    var supportDir: URL {
        let appSupport = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let dir = appSupport.appendingPathComponent("SiglusLauncher", isDirectory: true)
        if !fm.fileExists(atPath: dir.path) {
            try? fm.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        return dir
    }

    private var libraryURL: URL { supportDir.appendingPathComponent("library.json") }
    private var coverCacheDir: URL { supportDir.appendingPathComponent("covers", isDirectory: true) }

    var visibleGames: [GameEntry] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()

        var filtered = games

        if selectedEngineFilter != "all" {
            filtered = filtered.filter { $0.engine == selectedEngineFilter }
        }

        if !query.isEmpty {
            filtered = filtered.filter {
                $0.title.lowercased().contains(query) || $0.engineName.lowercased().contains(query)
            }
        }

        switch sortOption {
        case .lastPlayed:
            filtered.sort { a, b in
                let ap = a.lastPlayedAtUnix ?? 0
                let bp = b.lastPlayedAtUnix ?? 0
                if ap != bp { return ap > bp }
                return a.title.localizedStandardCompare(b.title) == .orderedAscending
            }
        case .title:
            filtered.sort { a, b in
                a.title.localizedStandardCompare(b.title) == .orderedAscending
            }
        case .added:
            filtered.sort { a, b in
                a.addedAtUnix > b.addedAtUnix
            }
        }

        return filtered
    }

    init() {
        load()
    }

    func load() {
        do {
            if fm.fileExists(atPath: libraryURL.path) {
                let data = try Data(contentsOf: libraryURL)
                games = try JSONDecoder().decode([GameEntry].self, from: data)
            } else {
                games = []
            }
        } catch {
            games = []
        }
        refreshValidation()
    }

    func save() {
        do {
            let data = try JSONEncoder().encode(games)
            try data.write(to: libraryURL, options: [.atomic])
        } catch {
            // best-effort
        }
    }

    func importGameFolders() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = true
        panel.prompt = "インポート"
        panel.title = "ゲームフォルダの選択"
        panel.message = "追加したいゲームフォルダ（複数選択可）、または複数のゲームが入った親フォルダを選択してください。"

        panel.begin { [weak self] response in
            guard let self else { return }
            if response != .OK { return }
            let urls = panel.urls
            Task { @MainActor in self.importFolders(urls) }
        }
    }

    func importFolders(_ urls: [URL]) {
        guard !urls.isEmpty else { return }
        isImporting = true
        let cacheDir = coverCacheDir.path
        DispatchQueue.global(qos: .userInitiated).async {
            var found: [(URL, [ProbedGame])] = []
            for url in urls {
                let json = url.path.withCString { path in
                    cacheDir.withCString { cache in
                        takeString(game_scan_json(path, 3, cache))
                    }
                }
                let games = json.flatMap { try? JSONDecoder().decode([ProbedGame].self, from: Data($0.utf8)) } ?? []
                found.append((url, games))
            }
            let result = found
            Task { @MainActor in
                self.finishImport(result)
            }
        }
    }

    private func finishImport(_ found: [(URL, [ProbedGame])]) {
        isImporting = false
        let now = Int64(Date().timeIntervalSince1970)
        var imported: [String] = []
        var updated: [String] = []
        var unsupported: [(String, String)] = []
        var empty: [String] = []
        for (url, probes) in found {
            if probes.isEmpty {
                empty.append(url.path)
                continue
            }
            for probe in probes {
                guard probe.supported else {
                    unsupported.append((probe.title, probe.unsupported_reason ?? "未対応のエンジンです"))
                    continue
                }
                let id = probe.id ?? stableId(for: probe.root)
                if let idx = games.firstIndex(where: { $0.id == id || $0.rootPath == probe.root }) {
                    apply(probe, to: &games[idx])
                    updated.append(probe.title)
                } else {
                    var entry = GameEntry(id: id, title: probe.title, rootPath: probe.root, addedAtUnix: now)
                    apply(probe, to: &entry)
                    games.append(entry)
                    imported.append(probe.title)
                }
            }
        }
        save()
        importReport = ImportReport(imported: imported, updated: updated, unsupported: unsupported, empty: empty)
    }

    private func apply(_ probe: ProbedGame, to entry: inout GameEntry) {
        entry.title = probe.title
        entry.rootPath = probe.root
        entry.coverPath = probe.cover
        entry.coverKind = probe.cover_kind
        entry.engine = probe.engine
        entry.engineName = probe.engine_name ?? probe.engine
        entry.nlsOptions = probe.nls_options ?? []
        if entry.nls == nil || !entry.nlsOptions.contains(where: { $0.id == entry.nls }) {
            entry.nls = probe.nls
        }
    }

    func reprobe(game: GameEntry) {
        guard let idx = games.firstIndex(where: { $0.id == game.id }) else { return }
        let cacheDir = coverCacheDir.path
        let json = game.rootPath.withCString { root in
            cacheDir.withCString { cache in
                if let nls = game.nls {
                    return nls.withCString { takeString(game_probe_json(root, $0, cache)) }
                }
                return takeString(game_probe_json(root, nil, cache))
            }
        }
        guard let json, let probe = try? JSONDecoder().decode(ProbedGame.self, from: Data(json.utf8)) else { return }
        apply(probe, to: &games[idx])
        save()
    }

    func refreshAll() {
        refreshValidation()
        for game in games { reprobe(game: game) }
    }

    func setNls(_ nls: String, for game: GameEntry) {
        guard let idx = games.firstIndex(where: { $0.id == game.id }) else { return }
        games[idx].nls = nls
        save()
        reprobe(game: games[idx])
        if selectedGameForDetail?.id == game.id {
            selectedGameForDetail = games[idx]
        }
    }

    func refreshValidation() {
        var changed = false
        games.removeAll { g in
            var isDir: ObjCBool = false
            let ok = fm.fileExists(atPath: g.rootPath, isDirectory: &isDir) && isDir.boolValue
            if !ok { changed = true }
            return !ok
        }
        if changed { save() }
    }

    func launch(game: GameEntry) {
        if let idx = games.firstIndex(where: { $0.id == game.id }) {
            games[idx].lastPlayedAtUnix = Int64(Date().timeIntervalSince1970)
            save()
        }

        guard let cb = onLaunchRequest else {
            showError("ランチャーホストが準備できていません。")
            return
        }
        cb(game)
    }

    func remove(game: GameEntry) {
        games.removeAll { $0.id == game.id }
        if selectedGameForDetail?.id == game.id {
            selectedGameForDetail = nil
        }
        save()
    }

    func revealInFinder(game: GameEntry) {
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: game.rootPath)])
    }

    func openLogsFolder() {
        let logsURL = fm.urls(for: .libraryDirectory, in: .userDomainMask).first!
            .appendingPathComponent("Logs/SiglusLauncher", isDirectory: true)
        try? fm.createDirectory(at: logsURL, withIntermediateDirectories: true)
        NSWorkspace.shared.open(logsURL)
    }

    func createKeyTomlTemplate(for game: GameEntry) {
        let url = game.keyTomlURL
        let template = """
# SiglusEngine リソース復号キー設定
# 体験版やキー不要の作品はオールゼロで動作します。
# 未設定の場合は自動でブルートフォース解析されます。

key = [
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
]

# 文字列暗号化の指定 ("xor", "none", "mdl")
# override_string_encryption = "xor"
"""
        do {
            try template.write(to: url, atomically: true, encoding: .utf8)
            if let idx = games.firstIndex(where: { $0.id == game.id }) {
                selectedGameForDetail = games[idx]
            }
        } catch {
            showError("key.toml の作成に失敗しました: \(error.localizedDescription)")
        }
    }

    func openKeyToml(for game: GameEntry) {
        let url = game.keyTomlURL
        if fm.fileExists(atPath: url.path) {
            NSWorkspace.shared.open(url)
        } else {
            createKeyTomlTemplate(for: game)
            NSWorkspace.shared.open(url)
        }
    }

    func loadCoverImage(game: GameEntry) -> NSImage? {
        guard let p = game.coverPath, fm.fileExists(atPath: p) else { return nil }
        return NSImage(contentsOfFile: p)
    }

    private func showError(_ msg: String) {
        errorMessage = msg
        showError = true
    }

    private func stableId(for path: String) -> String {
        var hash: UInt64 = 0xcbf2_9ce4_8422_2325
        for byte in path.utf8 {
            hash ^= UInt64(byte)
            hash = hash &* 0x0100_0000_01b3
        }
        return String(format: "%016llx", hash)
    }
}
