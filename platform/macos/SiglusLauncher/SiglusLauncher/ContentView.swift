import SwiftUI
import AppKit

struct ContentView: View {
    @EnvironmentObject var library: GameLibrary
    @State private var dropTargeted = false

    private let columns: [GridItem] = [
        GridItem(.adaptive(minimum: 220, maximum: 280), spacing: 18, alignment: .top),
    ]

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            filterAndToolbar
            Divider()
            ZStack {
                if library.games.isEmpty {
                    EmptyLibraryView()
                } else if library.visibleGames.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "magnifyingglass")
                            .font(.system(size: 40))
                            .foregroundColor(.secondary)
                        Text("一致するゲームが見つかりません")
                            .font(.headline)
                        Text("検索ワードまたはエンジンフィルターを変更してください。")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    ScrollView(.vertical) {
                        LazyVGrid(columns: columns, alignment: .leading, spacing: 18) {
                            ForEach(library.visibleGames) { game in
                                GameTileView(game: game)
                            }
                        }
                        .padding(20)
                    }
                }

                if dropTargeted {
                    RoundedRectangle(cornerRadius: 14)
                        .strokeBorder(Color.accentColor, style: StrokeStyle(lineWidth: 3, dash: [10, 6]))
                        .background(Color.accentColor.opacity(0.12))
                        .padding(12)
                        .overlay(
                            VStack(spacing: 8) {
                                Image(systemName: "plus.folder.fill")
                                    .font(.system(size: 48))
                                    .foregroundColor(.accentColor)
                                Text("ゲームフォルダをドロップして追加")
                                    .font(.title3)
                                    .bold()
                            }
                        )
                }

                if library.isImporting {
                    Color.black.opacity(0.3)
                    VStack(spacing: 12) {
                        ProgressView()
                            .scaleEffect(1.2)
                        Text("フォルダをスキャンして解析中…")
                            .font(.headline)
                    }
                    .padding(24)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Color(NSColor.windowBackgroundColor)))
                    .shadow(radius: 10)
                }
            }
            .onDrop(of: ["public.file-url"], isTargeted: $dropTargeted) { providers in
                loadDroppedFolders(providers)
                return true
            }

            footerStatusBar
        }
        .frame(minWidth: 840, minHeight: 560)
        .alert(isPresented: $library.showError) {
            Alert(
                title: Text("エラー"),
                message: Text(library.errorMessage),
                dismissButton: .default(Text("OK"))
            )
        }
        .sheet(item: $library.importReport) { report in
            ImportReportSheet(report: report)
        }
        .sheet(item: $library.selectedGameForDetail) { game in
            GameDetailSheet(game: game)
        }
    }

    private var header: some View {
        HStack(spacing: 14) {
            Image("Siglus")
                .resizable()
                .frame(width: 38, height: 38)
                .cornerRadius(8)

            VStack(alignment: .leading, spacing: 2) {
                Text("Siglus ランチャー")
                    .font(.title2)
                    .bold()
                Text("SiglusEngine · RealLive · AVG32 · UK2 完全対応")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            TextField("タイトルで検索…", text: $library.searchText)
                .textFieldStyle(RoundedBorderTextFieldStyle())
                .frame(width: 220)

            Button {
                library.importGameFolders()
            } label: {
                Label("ゲームを追加…", systemImage: "plus")
            }
            .keyboardShortcut("i", modifiers: [.command])
            .help("ゲームのフォルダ（複数可）またはまとめて入った親フォルダを追加")

            Button {
                library.refreshAll()
            } label: {
                Label("更新", systemImage: "arrow.clockwise")
            }
            .keyboardShortcut("r", modifiers: [.command])
            .help("ライブラリ内のゲーム情報を再読み込み")

            Menu {
                Button {
                    library.openLogsFolder()
                } label: {
                    Label("実行ログフォルダを開く", systemImage: "doc.text")
                }
                Button {
                    NSWorkspace.shared.activateFileViewerSelecting([library.supportDir])
                } label: {
                    Label("ライブラリデータフォルダを開く", systemImage: "folder")
                }
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuStyle(BorderlessButtonMenuStyle())
            .fixedSize()
            .help("その他")
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 12)
    }

    private var filterAndToolbar: some View {
        HStack(spacing: 16) {
            Picker("エンジンフィルター", selection: $library.selectedEngineFilter) {
                Text("すべて (\(library.games.count))").tag("all")
                Text("SiglusEngine").tag("siglus")
                Text("RealLive").tag("reallive")
                Text("AVG32").tag("avg32")
                Text("UK2").tag("uk2")
            }
            .pickerStyle(SegmentedPickerStyle())
            .frame(maxWidth: 420)

            Spacer()

            HStack(spacing: 6) {
                Text("並び順:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Picker("", selection: $library.sortOption) {
                    ForEach(GameSortOption.allCases) { opt in
                        Text(opt.rawValue).tag(opt)
                    }
                }
                .pickerStyle(MenuPickerStyle())
                .frame(width: 130)
            }
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 8)
        .background(Color(NSColor.windowBackgroundColor).opacity(0.6))
    }

    private var footerStatusBar: some View {
        HStack {
            Text("表示中: \(library.visibleGames.count) 本 / 登録済み: \(library.games.count) 本")
                .font(.caption2)
                .foregroundColor(.secondary)
            Spacer()
            Text("フォルダを直接ウィンドウにドラッグ＆ドロップして追加できます")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 6)
        .background(Color(NSColor.windowBackgroundColor))
    }

    private func loadDroppedFolders(_ providers: [NSItemProvider]) {
        let group = DispatchGroup()
        var urls: [URL] = []
        let lock = NSLock()
        for provider in providers {
            group.enter()
            provider.loadItem(forTypeIdentifier: "public.file-url", options: nil) { item, _ in
                defer { group.leave() }
                var url: URL? = nil
                if let data = item as? Data {
                    url = URL(dataRepresentation: data, relativeTo: nil)
                } else if let u = item as? URL {
                    url = u
                }
                if let url {
                    lock.lock()
                    urls.append(url)
                    lock.unlock()
                }
            }
        }
        group.notify(queue: .main) {
            Task { @MainActor in
                library.importFolders(urls)
            }
        }
    }
}

struct EmptyLibraryView: View {
    @EnvironmentObject var library: GameLibrary

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "square.stack.3d.up")
                .font(.system(size: 60))
                .foregroundColor(.accentColor.opacity(0.8))
            Text("ゲームがまだ登録されていません")
                .font(.title2)
                .bold()
            Text("ゲームのフォルダ（Gameexe.ini / Scene.pck / SEEN.TXT / UK2.CFG 等が含まれるフォルダ）を追加してください。\nエンジン種別（SiglusEngine, RealLive, AVG32, UK2）は自動判別されます。")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .frame(maxWidth: 500)
            Button {
                library.importGameFolders()
            } label: {
                Label("ゲームフォルダを追加…", systemImage: "folder.badge.plus")
                    .font(.headline)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
            }
            .keyboardShortcut(.defaultAction)

            Text("フォルダをここにドラッグ＆ドロップすることでも追加できます。")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(40)
    }
}

struct ImportReportSheet: View {
    @EnvironmentObject var library: GameLibrary
    let report: ImportReport

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Image(systemName: "info.circle.fill")
                    .foregroundColor(.accentColor)
                    .font(.title3)
                Text(report.title)
                    .font(.headline)
            }
            ScrollView {
                Text(report.message)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(8)
                    .background(Color(NSColor.textBackgroundColor))
                    .cornerRadius(6)
            }
            .frame(maxHeight: 280)

            HStack {
                Spacer()
                Button("閉じる") {
                    library.importReport = nil
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 520)
    }
}

struct EngineBadge: View {
    let engine: String
    let name: String

    private var color: Color {
        switch engine {
        case "siglus": return Color(red: 0.30, green: 0.45, blue: 0.95)
        case "reallive": return Color(red: 0.92, green: 0.45, blue: 0.20)
        case "avg32": return Color(red: 0.20, green: 0.68, blue: 0.45)
        case "uk2": return Color(red: 0.70, green: 0.35, blue: 0.82)
        default: return Color.gray
        }
    }

    var body: some View {
        Text(name)
            .font(.system(size: 10, weight: .bold))
            .foregroundColor(.white)
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(Capsule().fill(color.opacity(0.95)))
            .shadow(color: Color.black.opacity(0.2), radius: 2, y: 1)
    }
}

struct GameTileView: View {
    @EnvironmentObject var library: GameLibrary
    let game: GameEntry
    @State private var hovering = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Color.clear
                .aspectRatio(16.0 / 9.0, contentMode: .fit)
                .overlay(cover)
                .overlay(
                    ZStack {
                        Color.black.opacity(0.40)
                        VStack(spacing: 6) {
                            Image(systemName: "play.circle.fill")
                                .font(.system(size: 46))
                                .foregroundColor(.white)
                            Text("起動")
                                .font(.system(size: 13, weight: .bold))
                                .foregroundColor(.white)
                        }
                    }
                    .opacity(hovering ? 1 : 0)
                )
                .overlay(
                    EngineBadge(engine: game.engine, name: game.engineName).padding(8),
                    alignment: .topLeading
                )
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .contentShape(Rectangle())
                .onTapGesture { library.launch(game: game) }

            VStack(alignment: .leading, spacing: 4) {
                Text(game.title)
                    .font(.system(size: 13, weight: .semibold))
                    .lineLimit(1)
                    .truncationMode(.tail)

                HStack(spacing: 6) {
                    if let nlsLabel = game.nlsLabel {
                        Text(nlsLabel)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    } else {
                        Text(game.engineName)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    }

                    Spacer()

                    Text(game.lastPlayedString)
                        .font(.caption2)
                        .foregroundColor(.secondary)

                    Button {
                        library.selectedGameForDetail = game
                    } label: {
                        Image(systemName: "gearshape")
                            .font(.caption)
                    }
                    .buttonStyle(BorderlessButtonStyle())
                    .help("起動前設定・詳細")

                    Menu {
                        tileMenu
                    } label: {
                        Image(systemName: "ellipsis")
                            .font(.caption)
                    }
                    .menuStyle(BorderlessButtonMenuStyle())
                    .fixedSize()
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
        }
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(NSColor.controlBackgroundColor)))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color.accentColor.opacity(hovering ? 0.6 : 0), lineWidth: 1.5))
        .shadow(color: Color.black.opacity(hovering ? 0.18 : 0.05), radius: hovering ? 6 : 2, y: 1)
        .onHover { inside in
            withAnimation(.easeOut(duration: 0.12)) { hovering = inside }
        }
        .contextMenu { tileMenu }
        .help(game.title + "\n" + game.rootPath)
    }

    @ViewBuilder
    private var tileMenu: some View {
        Button {
            library.launch(game: game)
        } label: {
            Label("ゲームを起動", systemImage: "play.fill")
        }

        Button {
            library.selectedGameForDetail = game
        } label: {
            Label("設定・詳細…", systemImage: "gearshape")
        }

        Divider()

        if !game.nlsOptions.isEmpty {
            Menu("テキスト文字コード") {
                ForEach(game.nlsOptions, id: \.id) { option in
                    Button((option.id == game.nls ? "✓ " : "") + option.japaneseLabel) {
                        library.setNls(option.id, for: game)
                    }
                }
            }
        }

        if game.engine == "siglus" {
            Button {
                library.openKeyToml(for: game)
            } label: {
                Label(game.hasKeyToml ? "key.toml を開く" : "key.toml を生成…", systemImage: "key.fill")
            }
        }

        Divider()

        Button {
            library.revealInFinder(game: game)
        } label: {
            Label("Finderで表示", systemImage: "folder")
        }

        Button(role: .destructive) {
            library.remove(game: game)
        } label: {
            Label("ライブラリから削除", systemImage: "trash")
        }
    }

    @ViewBuilder
    private var cover: some View {
        if let image = library.loadCoverImage(game: game) {
            Image(nsImage: image)
                .resizable()
                .interpolation(game.coverKind == "icon" ? .none : .medium)
                .scaledToFill()
        } else {
            ZStack {
                LinearGradient(
                    gradient: Gradient(colors: [Color(red: 0.20, green: 0.22, blue: 0.30), Color(red: 0.08, green: 0.09, blue: 0.13)]),
                    startPoint: .top,
                    endPoint: .bottom
                )
                Text(game.title)
                    .font(.headline)
                    .foregroundColor(.white)
                    .multilineTextAlignment(.center)
                    .lineLimit(3)
                    .padding(12)
            }
        }
    }
}

/// ゲーム起動前の詳細・設定シート
struct GameDetailSheet: View {
    @EnvironmentObject var library: GameLibrary
    let game: GameEntry

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 16) {
                // カバー画像
                Color.clear
                    .aspectRatio(16.0 / 9.0, contentMode: .fit)
                    .frame(width: 180)
                    .overlay(coverThumbnail)
                    .clipShape(RoundedRectangle(cornerRadius: 8))

                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        EngineBadge(engine: game.engine, name: game.engineName)
                        Spacer()
                    }

                    Text(game.title)
                        .font(.title2)
                        .bold()
                        .lineLimit(2)

                    HStack(spacing: 12) {
                        Label("最終プレイ: \(game.lastPlayedString)", systemImage: "clock")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Label("登録日: \(game.addedDateString)", systemImage: "calendar")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(20)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    // フォルダパス
                    VStack(alignment: .leading, spacing: 6) {
                        Text("ゲームの保存場所")
                            .font(.subheadline)
                            .bold()
                        HStack {
                            Text(game.rootPath)
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .padding(6)
                                .background(Color(NSColor.controlBackgroundColor))
                                .cornerRadius(6)

                            Button("Finderで開く") {
                                library.revealInFinder(game: game)
                            }
                        }
                    }

                    // テキストエンコーディング (RealLive / AVG32 / UK2)
                    if !game.nlsOptions.isEmpty {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("テキスト文字コード (NLS)")
                                .font(.subheadline)
                                .bold()
                            Text("シナリオテキストの文字コードを設定します。日本語オリジナル版は Shift-JIS です。")
                                .font(.caption)
                                .foregroundColor(.secondary)

                            Picker("", selection: Binding(
                                get: { game.nls ?? "sjis" },
                                set: { library.setNls($0, for: game) }
                            )) {
                                ForEach(game.nlsOptions, id: \.id) { opt in
                                    Text(opt.japaneseLabel).tag(opt.id)
                                }
                            }
                            .pickerStyle(MenuPickerStyle())
                            .frame(maxWidth: 300)
                        }
                    }

                    // SiglusEngine 用 key.toml 設定支援
                    if game.engine == "siglus" {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("リソース復号キー (key.toml)")
                                .font(.subheadline)
                                .bold()

                            HStack(spacing: 8) {
                                if game.hasKeyToml {
                                    Label("key.toml が設定されています", systemImage: "checkmark.seal.fill")
                                        .font(.caption)
                                        .foregroundColor(.green)
                                    Spacer()
                                    Button("key.toml を編集") {
                                        library.openKeyToml(for: game)
                                    }
                                } else {
                                    Label("key.toml 未作成（起動時に自動ブルートフォース解析されます）", systemImage: "exclamationmark.circle")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                    Spacer()
                                    Button("key.toml を作成…") {
                                        library.createKeyTomlTemplate(for: game)
                                    }
                                }
                            }
                            .padding(10)
                            .background(Color(NSColor.controlBackgroundColor))
                            .cornerRadius(8)

                            Text("※ 体験版やキー不要の作品はオールゼロで動作します。手動でキーを設定したい場合は key.toml を作成・編集してください。")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                }
                .padding(20)
            }

            Divider()

            HStack(spacing: 12) {
                Button(role: .destructive) {
                    library.remove(game: game)
                } label: {
                    Label("ライブラリから削除", systemImage: "trash")
                }

                Spacer()

                Button("閉じる") {
                    library.selectedGameForDetail = nil
                }
                .keyboardShortcut(.cancelAction)

                Button {
                    let g = game
                    library.selectedGameForDetail = nil
                    library.launch(game: g)
                } label: {
                    Label("今すぐ起動", systemImage: "play.fill")
                        .font(.headline)
                        .padding(.horizontal, 10)
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.borderedProminent)
            }
            .padding(16)
            .background(Color(NSColor.windowBackgroundColor))
        }
        .frame(width: 580, height: 440)
    }

    @ViewBuilder
    private var coverThumbnail: some View {
        if let image = library.loadCoverImage(game: game) {
            Image(nsImage: image)
                .resizable()
                .scaledToFill()
        } else {
            ZStack {
                LinearGradient(
                    gradient: Gradient(colors: [Color(red: 0.20, green: 0.22, blue: 0.30), Color(red: 0.08, green: 0.09, blue: 0.13)]),
                    startPoint: .top,
                    endPoint: .bottom
                )
                Text(game.title)
                    .font(.caption)
                    .foregroundColor(.white)
                    .padding(6)
            }
        }
    }
}
