import Cocoa
import AVKit
import PDFKit
import Quartz
import ImageIO

struct Playlist: Decodable { let files: [String]; let shuffle: Bool; let office: Bool? }

// An audio-only surface underneath AVKit's native playback controls.
final class AudioBackdrop: NSView {
    var song = "" { didSet { needsDisplay = true } }
    var artist = "" { didSet { needsDisplay = true } }
    var album = "" { didSet { needsDisplay = true } }
    var artwork: NSImage? { didSet { needsDisplay = true } }
    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func draw(_ dirtyRect: NSRect) {
        let top = NSColor(calibratedRed: 0.10, green: 0.17, blue: 0.23, alpha: 1)
        let bottom = NSColor(calibratedRed: 0.05, green: 0.08, blue: 0.13, alpha: 1)
        NSGradient(starting: top, ending: bottom)?.draw(in: bounds, angle: 90)
        NSColor(calibratedRed: 0.24, green: 0.65, blue: 0.62, alpha: 0.06).setFill()
        NSBezierPath(ovalIn: NSRect(x: bounds.midX - bounds.width * 0.65, y: bounds.midY - bounds.width * 0.7, width: bounds.width * 1.3, height: bounds.width * 1.3)).fill()
        let side = max(64, min(280, min(bounds.width * 0.52, (bounds.height - 170) * 0.65)))
        let blockHeight = side + 116
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
    }
}

final class PreviewApp: NSObject, NSApplicationDelegate, NSWindowDelegate, NSTableViewDataSource, NSTableViewDelegate {
    var window: NSWindow!
    let player = AVPlayer()
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
    var failed = Set<URL>()

    func button(_ title: String, _ action: Selector) -> NSButton {
        NSButton(title: title, target: self, action: action)
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
            let playlist = try JSONDecoder().decode(Playlist.self, from: data)
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
    func windowWillClose(_ notification: Notification) { metadataAsset?.cancelLoading(); token = nil; player.pause(); player.replaceCurrentItem(with: nil); NSApp.terminate(nil) }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = PreviewApp()
app.delegate = delegate
let menu = NSMenu(); let appItem = NSMenuItem(); let appMenu = NSMenu()
appMenu.addItem(withTitle: "Quit Glim Preview", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
appItem.submenu = appMenu; menu.addItem(appItem)
let editItem = NSMenuItem(); let editMenu = NSMenu(title: "Edit")
editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
editMenu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
editItem.submenu = editMenu; menu.addItem(editItem); app.mainMenu = menu
app.run()
