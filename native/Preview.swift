import Cocoa
import AVKit
import PDFKit
import Quartz
import ImageIO
import UniformTypeIdentifiers
import CoreServices
import CoreText

struct Playlist: Decodable { let files: [String]; let shuffle: Bool; let office: Bool?; let associations: String?; let font: Bool? }

// Default handlers change only after a selection and explicit confirmation.
final class AssociationWindow: NSObject, NSWindowDelegate {
    let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 600), styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
    let bundleURL: URL
    let identifier: String
    let preferences = UserDefaults(suiteName: "io.github.jony4.glim.file-associations")!
    let groups: [(String, [String])] = [
        ("Markdown", ["md", "markdown"]), ("Plain Text", ["txt", "log"]),
        ("JSON", ["json", "jsonl", "geojson"]), ("YAML / TOML", ["yaml", "yml", "toml"]),
        ("HTML", ["html", "htm"]), ("XML", ["xml"]),
        ("Rust", ["rs"]), ("Python", ["py"]), ("JavaScript / TypeScript", ["js", "jsx", "ts", "tsx"]),
        ("C / C++", ["c", "h", "cpp", "hpp"]), ("Go / Swift", ["go", "swift"]),
        ("Java / Kotlin", ["java", "kt"]), ("Ruby / PHP", ["rb", "php"]),
        ("Shell / SQL", ["sh", "sql"]), ("CSS", ["css"]),
        ("Fonts", ["ttf", "otf", "ttc", "otc", "dfont"]),
        ("PDF", ["pdf"]), ("Images", ["png", "jpg", "jpeg", "webp", "gif", "svg", "bmp", "tiff"]),
        ("Audio", ["mp3", "m4a", "wav", "aac", "flac", "aiff", "caf"]),
        ("Video", ["mp4", "mov", "m4v"]),
        ("Keynote", ["key"]), ("Pages", ["pages"]), ("Numbers", ["numbers"]),
        ("Word", ["doc", "docx", "rtf"]), ("Excel", ["xls", "xlsx"]), ("PowerPoint", ["ppt", "pptx"])
    ]
    var checks: [NSButton] = []
    var currentLabels: [NSTextField] = []
    let status = NSTextField(wrappingLabelWithString: "Select formats to change. Other file types keep their current defaults.")
    init(bundleURL: URL) throws {
        self.bundleURL = bundleURL
        guard let bundle = Bundle(url: bundleURL), let identifier = bundle.bundleIdentifier,
              identifier == "io.github.jony4.glimpse" else { throw NSError(domain: "Glim", code: 1, userInfo: [NSLocalizedDescriptionKey: "Cannot identify the installed Glim app."]) }
        self.identifier = identifier
        super.init()
        window.title = "Default File Types — Glim"; window.minSize = NSSize(width: 560, height: 400); window.delegate = self; window.isReleasedWhenClosed = false
        let host = window.contentView!
        let stack = NSStackView(); stack.orientation = .vertical; stack.alignment = .leading; stack.spacing = 12; stack.translatesAutoresizingMaskIntoConstraints = false
        host.addSubview(stack)
        NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo: host.leadingAnchor, constant: 18), stack.trailingAnchor.constraint(equalTo: host.trailingAnchor, constant: -18), stack.topAnchor.constraint(equalTo: host.topAnchor, constant: 18), stack.bottomAnchor.constraint(equalTo: host.bottomAnchor, constant: -18)])
        let heading = NSTextField(labelWithString: "Open these file types with Glim"); heading.font = .systemFont(ofSize: 17, weight: .semibold); stack.addArrangedSubview(heading)
        let hint = NSTextField(wrappingLabelWithString: "macOS applies defaults to document types. Extensions belonging to the same type may change together.")
        hint.textColor = .secondaryLabelColor; stack.addArrangedSubview(hint)
        let list = NSStackView(); list.orientation = .vertical; list.alignment = .leading; list.spacing = 8; list.edgeInsets = NSEdgeInsets(top: 8, left: 0, bottom: 8, right: 8)
        for (title, extensions) in groups {
            let row = NSStackView(); row.orientation = .vertical; row.alignment = .leading; row.spacing = 3
            let check = NSButton(checkboxWithTitle: "\(title)  (\(extensions.map { "." + $0 }.joined(separator: ", ")))", target: nil, action: nil)
            let current = NSTextField(wrappingLabelWithString: ""); current.textColor = .secondaryLabelColor; current.font = .systemFont(ofSize: 11)
            row.addArrangedSubview(check); row.addArrangedSubview(current); list.addArrangedSubview(row)
            checks.append(check); currentLabels.append(current)
        }
        let scroll = NSScrollView(); scroll.hasVerticalScroller = true; scroll.documentView = list
        list.translatesAutoresizingMaskIntoConstraints = false
        list.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor).isActive = true
        stack.addArrangedSubview(scroll); scroll.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        status.font = .systemFont(ofSize: 12); stack.addArrangedSubview(status)
        let buttons = NSStackView()
        buttons.addArrangedSubview(NSButton(title: "Restore Selected", target: self, action: #selector(restore)))
        buttons.addArrangedSubview(NSButton(title: "Set Selected to Glim", target: self, action: #selector(apply)))
        stack.addArrangedSubview(buttons)
        refresh(); window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
    }
    func types(_ index: Int) -> [String] { Array(Set(groups[index].1.compactMap { UTType(filenameExtension: $0)?.identifier })).sorted() }
    func handler(_ type: String) -> String? {
        guard let value = LSCopyDefaultRoleHandlerForContentType(type as CFString, .all)?.takeRetainedValue() else { return nil }
        return value as String
    }
    func refresh() {
        for index in groups.indices {
            let names = Set(types(index).map { type -> String in
                guard let id = handler(type) else { return "Not set" }
                if id == identifier { return "Glim" }
                return NSWorkspace.shared.urlForApplication(withBundleIdentifier: id)?.deletingPathExtension().lastPathComponent ?? id
            })
            currentLabels[index].stringValue = "Current: " + (names.isEmpty ? "System type unavailable" : names.sorted().joined(separator: ", "))
        }
    }
    @objc func apply() { change(restore: false) }
    @objc func restore() { change(restore: true) }
    func change(restore: Bool) {
        let selected = groups.indices.filter { checks[$0].state == .on }
        guard !selected.isEmpty else { status.stringValue = "Select at least one format first."; return }
        let alert = NSAlert(); alert.messageText = restore ? "Restore previous default applications?" : "Use Glim as the default application?"
        alert.informativeText = selected.map { groups[$0].0 }.joined(separator: ", ")
        alert.addButton(withTitle: restore ? "Restore" : "Apply"); alert.addButton(withTitle: "Cancel")
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        guard LSRegisterURL(bundleURL as CFURL, true) == noErr else { status.stringValue = "macOS could not register Glim. Move it to Applications and retry."; return }
        var previous = preferences.dictionary(forKey: "previousHandlers") as? [String: String] ?? [:]
        let selectedTypes = Set(selected.flatMap { types($0) })
        var done = 0; var failures = 0
        for type in selectedTypes {
            let current = handler(type)
            let destination: String
            if restore {
                guard current == identifier, let old = previous[type] else { failures += 1; continue }
                destination = old
            } else {
                destination = identifier
                if let current = current, current != identifier { previous[type] = current }
            }
            let result = LSSetDefaultRoleHandlerForContentType(type as CFString, .all, destination as CFString)
            if result == noErr && handler(type) == destination { done += 1; if restore { previous.removeValue(forKey: type) } }
            else { failures += 1 }
        }
        preferences.set(previous, forKey: "previousHandlers")
        status.stringValue = "Updated \(done) document types." + (failures > 0 ? " \(failures) could not be changed or had no saved default. You can also use Finder → Get Info → Open with → Change All." : "")
        refresh()
    }
    func windowWillClose(_ notification: Notification) { NSApp.terminate(nil) }
}

final class FontPreview: NSObject, NSTextFieldDelegate {
    let view = NSView()
    let selector = NSPopUpButton()
    let input = NSTextField(string: "The quick brown fox jumps over the lazy dog. 0123456789 中文字体预览")
    let metadata = NSTextField(wrappingLabelWithString: "Loading font…")
    var descriptors: [CTFontDescriptor] = []
    let sizes: [CGFloat] = [12, 18, 24, 36, 48, 72]
    var samples: [NSTextField] = []
    var availableFaces = 0
    init(url: URL) {
        super.init()
        let stack = NSStackView(); stack.orientation = .vertical; stack.alignment = .leading; stack.spacing = 12; stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 20), stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -20), stack.topAnchor.constraint(equalTo: view.topAnchor, constant: 16), stack.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -16)])
        let title = NSTextField(labelWithString: url.lastPathComponent); title.font = .systemFont(ofSize: 17, weight: .semibold); stack.addArrangedSubview(title)
        selector.target = self; selector.action = #selector(changeFace); selector.isEnabled = false; stack.addArrangedSubview(selector)
        input.delegate = self; input.placeholderString = "Type sample text…"; stack.addArrangedSubview(input)
        input.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        metadata.textColor = .secondaryLabelColor; metadata.font = .systemFont(ofSize: 12); stack.addArrangedSubview(metadata)
        let rows = NSStackView(); rows.orientation = .vertical; rows.alignment = .leading; rows.spacing = 20; rows.edgeInsets = NSEdgeInsets(top: 16, left: 0, bottom: 16, right: 12)
        for size in sizes {
            let group = NSStackView(); group.orientation = .vertical; group.alignment = .leading; group.spacing = 6
            let caption = NSTextField(labelWithString: "\(Int(size)) pt"); caption.textColor = .secondaryLabelColor; caption.font = .systemFont(ofSize: 11)
            let sample = NSTextField(wrappingLabelWithString: ""); sample.isSelectable = true
            group.addArrangedSubview(caption); group.addArrangedSubview(sample); rows.addArrangedSubview(group); samples.append(sample)
            group.widthAnchor.constraint(equalTo: rows.widthAnchor).isActive = true
            sample.widthAnchor.constraint(equalTo: group.widthAnchor).isActive = true
        }
        let scroll = NSScrollView(); scroll.hasVerticalScroller = true; scroll.documentView = rows
        rows.translatesAutoresizingMaskIntoConstraints = false; rows.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor).isActive = true
        stack.addArrangedSubview(scroll); scroll.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        let note = NSTextField(wrappingLabelWithString: "Preview only — this font is not installed. Missing characters may use a system fallback."); note.font = .systemFont(ofSize: 11); note.textColor = .secondaryLabelColor; stack.addArrangedSubview(note)
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let fonts = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] ?? []
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.availableFaces = fonts.count
                self.descriptors = Array(fonts.prefix(128))
                guard !self.descriptors.isEmpty else { self.metadata.stringValue = "This font could not be decoded. It may be damaged or unsupported."; return }
                for descriptor in self.descriptors {
                    let font = CTFontCreateWithFontDescriptor(descriptor, 16, nil)
                    self.selector.addItem(withTitle: CTFontCopyFullName(font) as String)
                }
                self.selector.isEnabled = self.descriptors.count > 1
                self.changeFace()
            }
        }
    }
    @objc func changeFace() {
        let index = selector.indexOfSelectedItem
        guard descriptors.indices.contains(index) else { return }
        let font = CTFontCreateWithFontDescriptor(descriptors[index], 16, nil)
        metadata.stringValue = "\(CTFontCopyFamilyName(font)) · \(CTFontCopyPostScriptName(font)) · \(CTFontGetGlyphCount(font)) glyphs · \(availableFaces) faces" + (availableFaces > 128 ? " (showing first 128)" : "")
        updateSamples()
    }
    func controlTextDidChange(_ notification: Notification) { updateSamples() }
    func updateSamples() {
        let index = selector.indexOfSelectedItem
        guard descriptors.indices.contains(index) else { return }
        let text = String(input.stringValue.prefix(512))
        for (label, size) in zip(samples, sizes) {
            label.font = CTFontCreateWithFontDescriptor(descriptors[index], size, nil) as NSFont
            label.stringValue = text
        }
    }
}

// An audio-only surface underneath AVKit's native playback controls.
final class AudioBackdrop: NSView {
    var song = "" { didSet { needsDisplay = true } }
    var artist = "" { didSet { needsDisplay = true } }
    var album = "" { didSet { needsDisplay = true } }
    var artwork: NSImage? { didSet { needsDisplay = true } }
    var motionTime: Double = 0
    var playing = false
    override var isFlipped: Bool { true }
    override var wantsDefaultClipping: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func draw(_ dirtyRect: NSRect) {
        NSGraphicsContext.saveGraphicsState()
        defer { NSGraphicsContext.restoreGraphicsState() }
        NSBezierPath(rect: bounds).addClip()
        let top = NSColor(calibratedRed: 0.10, green: 0.17, blue: 0.23, alpha: 1)
        let bottom = NSColor(calibratedRed: 0.05, green: 0.08, blue: 0.13, alpha: 1)
        NSGradient(starting: top, ending: bottom)?.draw(in: bounds, angle: 90)
        let side = max(64, min(280, min(bounds.width * 0.52, (bounds.height - 170) * 0.65)))
        let blockHeight = side + 158
        let y = max(24, (bounds.height - blockHeight) / 2)
        let cover = NSRect(x: bounds.midX - side / 2, y: y, width: side, height: side)
        NSGraphicsContext.saveGraphicsState()
        let shadow = NSShadow(); shadow.shadowColor = NSColor.black.withAlphaComponent(0.3); shadow.shadowBlurRadius = 22; shadow.shadowOffset = NSSize(width: 0, height: 8); shadow.set()
        NSColor(calibratedWhite: 0.12, alpha: 1).setFill()
        NSBezierPath(roundedRect: cover, xRadius: 14, yRadius: 14).fill()
        NSGraphicsContext.restoreGraphicsState()
        if let artwork = artwork {
            NSGraphicsContext.saveGraphicsState()
            NSBezierPath(roundedRect: cover, xRadius: 14, yRadius: 14).addClip()
            let size = artwork.size
            let scale = max(side / max(1, size.width), side / max(1, size.height))
            let rect = NSRect(x: cover.midX - size.width * scale / 2, y: cover.midY - size.height * scale / 2, width: size.width * scale, height: size.height * scale)
            artwork.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
            NSGraphicsContext.restoreGraphicsState()
        } else {
            let disc = cover.insetBy(dx: side * 0.06, dy: side * 0.06)
            NSColor(calibratedWhite: 0.055, alpha: 1).setFill(); NSBezierPath(ovalIn: disc).fill()
            NSColor.white.withAlphaComponent(0.08).setStroke()
            for fraction in [CGFloat(0.08), 0.14, 0.20, 0.26] {
                NSBezierPath(ovalIn: disc.insetBy(dx: side * fraction, dy: side * fraction)).stroke()
            }
            NSColor(calibratedRed: 0.39, green: 0.78, blue: 0.72, alpha: 1).setFill()
            NSBezierPath(ovalIn: cover.insetBy(dx: side * 0.35, dy: side * 0.35)).fill()
            NSColor(calibratedWhite: 0.1, alpha: 1).setFill()
            NSBezierPath(ovalIn: NSRect(x: cover.midX - 4, y: cover.midY - 4, width: 8, height: 8)).fill()
        }
        let style = NSMutableParagraphStyle(); style.alignment = .center; style.lineBreakMode = .byTruncatingTail
        func label(_ text: String, _ offset: CGFloat, _ size: CGFloat, _ color: NSColor, _ weight: NSFont.Weight) {
            (text as NSString).draw(in: NSRect(x: 28, y: cover.maxY + offset, width: max(1, bounds.width - 56), height: 30), withAttributes: [.font: NSFont.systemFont(ofSize: size, weight: weight), .foregroundColor: color, .paragraphStyle: style])
        }
        label(song, 22, 22, .white, .semibold)
        label(artist, 54, 15, NSColor.white.withAlphaComponent(0.72), .regular)
        label(album, 79, 12, NSColor.white.withAlphaComponent(0.45), .regular)
        // A decorative playback indicator, driven by playback time rather than
        // fabricated frequency/amplitude measurements. Respect Reduce Motion.
        let animate = playing && !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        let count = 29
        let spacing: CGFloat = min(8, max(4, (bounds.width - 64) / CGFloat(count)))
        let start = bounds.midX - CGFloat(count - 1) * spacing / 2
        let centerY = min(bounds.height - 84, cover.maxY + 126)
        if centerY > cover.maxY + 100 {
            for index in 0..<count {
                let position = Double(index) / Double(count - 1)
                let envelope = sin(position * .pi)
                let wave = (sin(motionTime * 4.1 + Double(index) * 0.72)
                    + sin(motionTime * 2.7 - Double(index) * 0.41) + 2) / 4
                let height: CGFloat = animate ? CGFloat(4 + 25 * envelope * wave) : CGFloat(3 + 3 * envelope)
                NSColor(calibratedRed: 0.39, green: 0.78, blue: 0.72, alpha: animate ? 0.68 : 0.25).setFill()
                NSBezierPath(roundedRect: NSRect(x: start + CGFloat(index) * spacing - 1.5,
                    y: centerY - height / 2, width: 3, height: height), xRadius: 1.5, yRadius: 1.5).fill()
            }
        }
    }
}

final class PreviewApp: NSObject, NSApplicationDelegate, NSWindowDelegate, NSTableViewDataSource, NSTableViewDelegate {
    var window: NSWindow!
    var associationWindow: AssociationWindow?
    let player = AVPlayer()
    var playbackObserver: Any?
    var playbackStateObserver: NSKeyValueObservation?
    let playerView = AVPlayerView()
    let pdfView = PDFView()
    let audioBackdrop = AudioBackdrop()
    var metadataAsset: AVURLAsset?
    let titleLabel = NSTextField(labelWithString: "")
    let detailLabel = NSTextField(labelWithString: "")
    let table = NSTableView()
    var urls: [URL] = []
    var index = 0
    var token: NSKeyValueObservation?
    var pdf = false
    var officeView: QLPreviewView?
    var fontPreview: FontPreview?
    var failed = Set<URL>()

    func button(_ title: String, _ action: Selector) -> NSButton {
        NSButton(title: title, target: self, action: action)
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
            let playlist = try JSONDecoder().decode(Playlist.self, from: data)
            if let path = playlist.associations {
                associationWindow = try AssociationWindow(bundleURL: URL(fileURLWithPath: path))
                window = associationWindow?.window
                return
            }
            urls = playlist.files.map { URL(fileURLWithPath: $0) }
                .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
            if playlist.shuffle { urls.shuffle() }
            guard !urls.isEmpty else { throw NSError(domain: "Glim", code: 1, userInfo: [NSLocalizedDescriptionKey: "No media files found."]) }
            pdf = urls.count == 1 && urls[0].pathExtension.lowercased() == "pdf"
            window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1000, height: 740), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
            window.title = "Glim"
            window.minSize = NSSize(width: 640, height: 420)
            window.delegate = self
            window.isReleasedWhenClosed = false
            if playlist.font == true {
                let preview = FontPreview(url: urls[0]); fontPreview = preview
                preview.view.frame = window.contentView!.bounds
                preview.view.autoresizingMask = [.width, .height]
                window.contentView!.addSubview(preview.view)
                window.title = "\(urls[0].lastPathComponent) — Glim"
                window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
                return
            }
            if playlist.office == true {
                showOffice(urls[0])
                window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
                return
            }
            let content = NSStackView()
            content.orientation = .vertical
            content.spacing = 0
            content.translatesAutoresizingMaskIntoConstraints = false
            window.contentView!.addSubview(content)
            NSLayoutConstraint.activate([
                content.leadingAnchor.constraint(equalTo: window.contentView!.leadingAnchor), content.trailingAnchor.constraint(equalTo: window.contentView!.trailingAnchor),
                content.topAnchor.constraint(equalTo: window.contentView!.topAnchor), content.bottomAnchor.constraint(equalTo: window.contentView!.bottomAnchor)
            ])
            titleLabel.lineBreakMode = .byTruncatingMiddle
            titleLabel.font = NSFont.systemFont(ofSize: 13, weight: .semibold)
            detailLabel.textColor = .secondaryLabelColor
            let toolbar = NSStackView()
            toolbar.edgeInsets = NSEdgeInsets(top: 10, left: 12, bottom: 10, right: 12)
            toolbar.addArrangedSubview(button("Previous", #selector(previous)))
            toolbar.addArrangedSubview(button("Next", #selector(next)))
            if pdf {
                toolbar.addArrangedSubview(button("−", #selector(zoomOut)))
                toolbar.addArrangedSubview(button("+", #selector(zoomIn)))
                toolbar.addArrangedSubview(button("Fit", #selector(fit)))
            } else {
                toolbar.addArrangedSubview(button("Play / Pause", #selector(togglePlay)))
            }
            toolbar.addArrangedSubview(titleLabel)
            toolbar.addArrangedSubview(detailLabel)
            content.addArrangedSubview(toolbar)
            toolbar.widthAnchor.constraint(equalTo: content.widthAnchor).isActive = true
            if pdf {
                guard let document = PDFDocument(url: urls[0]) else { throw NSError(domain: "Glim", code: 2, userInfo: [NSLocalizedDescriptionKey: "This PDF could not be opened."]) }
                if document.isLocked {
                    let alert = NSAlert(); alert.messageText = "PDF password"; alert.addButton(withTitle: "Open"); alert.addButton(withTitle: "Cancel")
                    let password = NSSecureTextField(frame: NSRect(x: 0, y: 0, width: 260, height: 24)); alert.accessoryView = password
                    guard alert.runModal() == .alertFirstButtonReturn && document.unlock(withPassword: password.stringValue) else { NSApp.terminate(nil); return }
                }
                pdfView.document = document; pdfView.autoScales = true; pdfView.displayMode = .singlePageContinuous
                content.addArrangedSubview(pdfView)
                pdfView.widthAnchor.constraint(equalTo: content.widthAnchor).isActive = true
                NotificationCenter.default.addObserver(self, selector: #selector(pageChanged), name: .PDFViewPageChanged, object: pdfView)
                titleLabel.stringValue = urls[0].lastPathComponent; pageChanged()
            } else {
                playerView.player = player
                playbackObserver = player.addPeriodicTimeObserver(forInterval: CMTime(seconds: 1.0 / 24.0, preferredTimescale: 600), queue: .main) { [weak self] time in
                    guard let self = self, !self.audioBackdrop.isHidden,
                          self.window.occlusionState.contains(.visible),
                          !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion else { return }
                    let seconds = time.seconds
                    if seconds.isFinite { self.audioBackdrop.motionTime = seconds }
                    self.audioBackdrop.needsDisplay = true
                }
                playbackStateObserver = player.observe(\.timeControlStatus, options: [.initial, .new]) { [weak self] player, _ in
                    DispatchQueue.main.async {
                        self?.audioBackdrop.playing = player.timeControlStatus == .playing
                        self?.audioBackdrop.needsDisplay = true
                    }
                }
                playerView.controlsStyle = .inline
                if let overlay = playerView.contentOverlayView {
                    audioBackdrop.frame = overlay.bounds
                    audioBackdrop.autoresizingMask = [.width, .height]
                    audioBackdrop.isHidden = true
                    audioBackdrop.setAccessibilityElement(true)
                    audioBackdrop.setAccessibilityRole(.image)
                    overlay.addSubview(audioBackdrop)
                }
                let body = NSStackView(); body.orientation = .horizontal; body.spacing = 0
                body.addArrangedSubview(playerView)
                playerView.setContentHuggingPriority(.defaultLow, for: .horizontal)
                if urls.count > 1 {
                    let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("file")); column.title = "Playlist"; table.addTableColumn(column)
                    table.headerView = nil; table.delegate = self; table.dataSource = self; table.rowHeight = 28
                    let scroll = NSScrollView(); scroll.documentView = table; scroll.hasVerticalScroller = true
                    scroll.widthAnchor.constraint(equalToConstant: 240).isActive = true
                    body.addArrangedSubview(scroll)
                    table.reloadData()
                }
                content.addArrangedSubview(body)
                body.widthAnchor.constraint(equalTo: content.widthAnchor).isActive = true
                NotificationCenter.default.addObserver(self, selector: #selector(finished(_:)), name: .AVPlayerItemDidPlayToEndTime, object: nil)
                NotificationCenter.default.addObserver(self, selector: #selector(playbackFailed(_:)), name: .AVPlayerItemFailedToPlayToEndTime, object: nil)
                play(0)
            }
            window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        } catch {
            let alert = NSAlert(error: error); alert.runModal(); NSApp.terminate(nil)
        }
    }
    func showOffice(_ url: URL) {
        window.title = "\(url.lastPathComponent) — Glim"
        let host = window.contentView!
        let toolbar = NSStackView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false
        toolbar.spacing = 12
        titleLabel.stringValue = url.lastPathComponent
        titleLabel.lineBreakMode = .byTruncatingMiddle
        titleLabel.font = NSFont.systemFont(ofSize: 13, weight: .semibold)
        toolbar.addArrangedSubview(titleLabel)
        toolbar.addArrangedSubview(button("Open in Default App", #selector(openOfficeExternally)))
        host.addSubview(toolbar)
        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: host.topAnchor, constant: 10),
            toolbar.leadingAnchor.constraint(equalTo: host.leadingAnchor, constant: 12),
            toolbar.trailingAnchor.constraint(equalTo: host.trailingAnchor, constant: -12)
        ])
        guard let preview = QLPreviewView(frame: host.bounds, style: .normal) else {
            let alert = NSAlert(); alert.messageText = "Document preview is unavailable"; alert.informativeText = "Open this document in its default app to view it."; alert.runModal(); return
        }
        officeView = preview
        preview.translatesAutoresizingMaskIntoConstraints = false
        preview.shouldCloseWithWindow = true
        preview.autostarts = true
        host.addSubview(preview)
        NSLayoutConstraint.activate([
            preview.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 10),
            preview.leadingAnchor.constraint(equalTo: host.leadingAnchor),
            preview.trailingAnchor.constraint(equalTo: host.trailingAnchor),
            preview.bottomAnchor.constraint(equalTo: host.bottomAnchor)
        ])
        preview.previewItem = url as NSURL
    }
    @objc func openOfficeExternally() { if let url = urls.first { NSWorkspace.shared.open(url) } }

    func play(_ next: Int) {
        guard urls.indices.contains(next) else { player.pause(); return }
        token = nil; index = next
        let item = AVPlayerItem(url: urls[index])
        player.replaceCurrentItem(with: item)
        updateAudioArtwork(urls[index], item: item)
        titleLabel.stringValue = urls[index].lastPathComponent
        detailLabel.stringValue = "\(index + 1) / \(urls.count)"
        window.title = "\(urls[index].lastPathComponent) — Glim"
        if urls.count > 1 {
            table.selectRowIndexes(IndexSet(integer: index), byExtendingSelection: false)
            table.scrollRowToVisible(index)
        }
        token = item.observe(\.status, options: [.initial, .new]) { [weak self] item, _ in
            DispatchQueue.main.async {
                guard let self = self, self.player.currentItem === item else { return }
                if item.status == .failed { self.skipFailed(item) }
            }
        }
        player.play()
    }
    func updateAudioArtwork(_ url: URL, item: AVPlayerItem) {
        metadataAsset?.cancelLoading()
        audioBackdrop.artwork = nil
        audioBackdrop.song = url.deletingPathExtension().lastPathComponent
        audioBackdrop.artist = ""
        audioBackdrop.album = url.pathExtension.uppercased() + " · Audio"
        audioBackdrop.isHidden = !["mp3", "m4a", "aac", "wav", "aif", "aiff", "caf", "flac"].contains(url.pathExtension.lowercased())
        let asset = AVURLAsset(url: url)
        metadataAsset = asset
        asset.loadValuesAsynchronously(forKeys: ["commonMetadata", "tracks"]) { [weak self, weak item] in
            var error: NSError?
            let loadedTracks = asset.statusOfValue(forKey: "tracks", error: &error) == .loaded
            let audioOnly = loadedTracks && asset.tracks(withMediaType: .video).isEmpty
            let metadata = asset.statusOfValue(forKey: "commonMetadata", error: &error) == .loaded ? asset.commonMetadata : []
            func value(_ key: AVMetadataKey) -> String? {
                AVMetadataItem.metadataItems(from: metadata, withKey: key, keySpace: .common).first?.stringValue
            }
            let song = value(.commonKeyTitle)
            let artist = value(.commonKeyArtist)
            let album = value(.commonKeyAlbumName)
            var image: CGImage?
            if let data = AVMetadataItem.metadataItems(from: metadata, withKey: AVMetadataKey.commonKeyArtwork, keySpace: .common).first?.dataValue,
                data.count <= 8 * 1024 * 1024,
                let source = CGImageSourceCreateWithData(data as CFData, nil) {
                image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 600, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary)
            }
            DispatchQueue.main.async {
                guard let self = self, let item = item, self.player.currentItem === item else { return }
                if loadedTracks { self.audioBackdrop.isHidden = !audioOnly }
                if let song = song, !song.isEmpty { self.audioBackdrop.song = song }
                self.audioBackdrop.artist = artist ?? ""
                if let album = album, !album.isEmpty { self.audioBackdrop.album = album }
                if let image = image { self.audioBackdrop.artwork = NSImage(cgImage: image, size: .zero) }
                self.audioBackdrop.setAccessibilityLabel([self.audioBackdrop.song, self.audioBackdrop.artist, self.audioBackdrop.album].filter { !$0.isEmpty }.joined(separator: ", "))
            }
        }
    }

    func skipFailed(_ item: AVPlayerItem) {
        guard player.currentItem === item, failed.insert(urls[index]).inserted else { return }
        detailLabel.stringValue = "Cannot play this format or codec"
        if index + 1 < urls.count { play(index + 1) }
        else { player.pause() }
    }
    @objc func playbackFailed(_ note: Notification) { if let item = note.object as? AVPlayerItem { skipFailed(item) } }
    @objc func finished(_ note: Notification) { if let item = note.object as? AVPlayerItem, item === player.currentItem { next() } }
    @objc func next() { if pdf { pdfView.goToNextPage(nil) } else { play(index + 1) } }
    @objc func previous() { if pdf { pdfView.goToPreviousPage(nil) } else { play(max(0, index - 1)) } }
    @objc func togglePlay() { if player.rate == 0 { player.play() } else { player.pause() } }
    @objc func zoomIn() { pdfView.zoomIn(nil) }
    @objc func zoomOut() { pdfView.zoomOut(nil) }
    @objc func fit() { pdfView.autoScales = true }
    @objc func pageChanged() { if let document = pdfView.document, let page = pdfView.currentPage { detailLabel.stringValue = "\(document.index(for: page) + 1) / \(document.pageCount)" } }
    func numberOfRows(in tableView: NSTableView) -> Int { urls.count }
    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let label = NSTextField(labelWithString: urls[row].lastPathComponent); label.lineBreakMode = .byTruncatingMiddle; label.toolTip = urls[row].lastPathComponent; return label
    }
    func tableViewSelectionDidChange(_ notification: Notification) { if table.selectedRow >= 0 && table.selectedRow != index { play(table.selectedRow) } }
    func windowWillClose(_ notification: Notification) {
        metadataAsset?.cancelLoading(); token = nil; playbackStateObserver = nil
        if let observer = playbackObserver { player.removeTimeObserver(observer); playbackObserver = nil }
        player.pause(); player.replaceCurrentItem(with: nil); NSApp.terminate(nil)
    }
    func showPreviewWindow() {
        guard let window = window else { return }
        NSApp.unhide(nil)
        if window.isMiniaturized { window.deminiaturize(nil) }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        showPreviewWindow()
        return true
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = PreviewApp()
app.delegate = delegate
FileHandle.standardInput.readabilityHandler = { [weak delegate] handle in
    let data = handle.availableData
    if data.isEmpty { handle.readabilityHandler = nil; return }
    if String(data: data, encoding: .utf8)?.contains("show") == true {
        DispatchQueue.main.async { delegate?.showPreviewWindow() }
    }
}
let menu = NSMenu(); let appItem = NSMenuItem(); let appMenu = NSMenu()
appMenu.addItem(withTitle: "Quit Glim Preview", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
appItem.submenu = appMenu; menu.addItem(appItem)
let editItem = NSMenuItem(); let editMenu = NSMenu(title: "Edit")
editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
editMenu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
editItem.submenu = editMenu; menu.addItem(editItem); app.mainMenu = menu
app.run()
