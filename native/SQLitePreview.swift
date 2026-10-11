import Cocoa
import SQLite3

final class SQLitePreview: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    let view = NSView()
    let picker = NSPopUpButton()
    let grid = NSTableView()
    let status = NSTextField(wrappingLabelWithString: "Loading database…")
    let previous = NSButton(title: "Previous", target: nil, action: nil)
    let next = NSButton(title: "Next", target: nil, action: nil)
    let structure = NSButton(checkboxWithTitle: "Structure", target: nil, action: nil)
    let schemaButton = NSButton(title: "SQL", target: nil, action: nil)
    private let queue = DispatchQueue(label: "glim.sqlite", qos: .userInitiated)
    private var database: OpaquePointer?
    private var tables: [(name: String, sql: String)] = []
    private var rows: [[String]] = []
    private var offset = 0
    private var more = false
    private var busy = false
    private let pageSize = 200

    init(path: String?, information: String) {
        super.init()
        let stack = NSStackView(); stack.orientation = .vertical; stack.alignment = .leading; stack.spacing = 10; stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16), stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16), stack.topAnchor.constraint(equalTo: view.topAnchor, constant: 16), stack.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -16)])
        let note = NSTextField(wrappingLabelWithString: information); note.textColor = .secondaryLabelColor; stack.addArrangedSubview(note)
        let toolbar = NSStackView()
        picker.target = self; picker.action = #selector(selectTable); picker.isEnabled = false
        previous.target = self; previous.action = #selector(previousPage)
        next.target = self; next.action = #selector(nextPage)
        structure.target = self; structure.action = #selector(selectTable)
        schemaButton.target = self; schemaButton.action = #selector(showSQL)
        [picker, structure, schemaButton, previous, next].forEach { toolbar.addArrangedSubview($0) }; stack.addArrangedSubview(toolbar)
        grid.delegate = self; grid.dataSource = self; grid.usesAlternatingRowBackgroundColors = true; grid.rowHeight = 26; grid.columnAutoresizingStyle = .noColumnAutoresizing
        let scroll = NSScrollView(); scroll.documentView = grid; scroll.hasVerticalScroller = true; scroll.hasHorizontalScroller = true
        stack.addArrangedSubview(scroll); scroll.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        status.font = .systemFont(ofSize: 12); status.textColor = .secondaryLabelColor; stack.addArrangedSubview(status)
        controls()
        guard let path = path else { status.stringValue = "Open the matching database to browse tables."; return }
        queue.async { [self] in
            // Only the private snapshot is writable, to allow journal/WAL recovery.
            let code = sqlite3_open_v2(path, &database, SQLITE_OPEN_READWRITE | SQLITE_OPEN_PRIVATECACHE, nil)
            guard code == SQLITE_OK, let database = database else { fail("Cannot open SQLite snapshot."); return }
            sqlite3_busy_timeout(database, 1000)
            sqlite3_limit(database, SQLITE_LIMIT_LENGTH, 8 * 1024 * 1024)
            sqlite3_limit(database, SQLITE_LIMIT_COLUMN, 256)
            // macOS system SQLite omits the extension-loading API.
            sqlite3_exec(database, "PRAGMA trusted_schema=OFF; PRAGMA query_only=ON; PRAGMA cache_size=-8192; PRAGMA temp_store=FILE;", nil, nil, nil)
            do {
                let result = try query("SELECT name, COALESCE(sql, '') FROM sqlite_master WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 1001", limit: 1001, textLimit: 16384)
                let values = result.rows.prefix(1000).map { (name: $0[0], sql: $0[1]) }
                DispatchQueue.main.async { [weak self] in
                    guard let self = self else { return }
                    self.tables = values
                    self.picker.addItems(withTitles: values.map { $0.name })
                    self.status.stringValue = values.isEmpty ? "No user tables or views." : "\(values.count) tables / views"
                    if result.rows.count > 1000 { self.status.stringValue += " (first 1,000 shown)" }
                    self.controls()
                    if !values.isEmpty { self.loadPage() }
                }
            } catch { fail(error.localizedDescription) }
        }
    }
    deinit { if let database = database { sqlite3_close(database) } }
    private func failure() -> NSError {
        NSError(domain: "SQLite", code: Int(sqlite3_errcode(database)), userInfo: [NSLocalizedDescriptionKey: database.map { String(cString: sqlite3_errmsg($0)) } ?? "Database unavailable"])
    }
    private func query(_ sql: String, limit: Int, textLimit: Int = 512) throws -> (columns: [String], rows: [[String]]) {
        guard let database = database else { throw failure() }
        var deadline = CFAbsoluteTimeGetCurrent() + 5
        return try withUnsafeMutablePointer(to: &deadline) { pointer in
            sqlite3_progress_handler(database, 1000, { context in
                guard let context = context else { return 1 }
                return CFAbsoluteTimeGetCurrent() > context.assumingMemoryBound(to: Double.self).pointee ? 1 : 0
            }, pointer)
            defer { sqlite3_progress_handler(database, 0, nil, nil) }
            var statement: OpaquePointer?
            guard sqlite3_prepare_v2(database, sql, -1, &statement, nil) == SQLITE_OK, let statement = statement else { throw failure() }
            defer { sqlite3_finalize(statement) }
            guard sqlite3_stmt_readonly(statement) != 0 else { throw NSError(domain: "SQLite", code: 1, userInfo: [NSLocalizedDescriptionKey: "Only read-only queries are allowed."]) }
            let count = min(Int(sqlite3_column_count(statement)), 64)
            let columns = (0..<count).map { String(cString: sqlite3_column_name(statement, Int32($0))) }
            var result: [[String]] = []
            while result.count < limit {
                let code = sqlite3_step(statement)
                if code == SQLITE_DONE { break }
                guard code == SQLITE_ROW else { throw failure() }
                result.append((0..<count).map { column in
                    let index = Int32(column)
                    if sqlite3_column_type(statement, index) == SQLITE_NULL { return "NULL" }
                    if sqlite3_column_type(statement, index) == SQLITE_BLOB {
                        let length = Int(sqlite3_column_bytes(statement, index))
                        guard let bytes = sqlite3_column_blob(statement, index)?.assumingMemoryBound(to: UInt8.self) else { return "BLOB · 0 bytes" }
                        let prefix = UnsafeBufferPointer(start: bytes, count: min(length, 16)).map { String(format: "%02x", $0) }.joined(separator: " ")
                        return "BLOB · \(length) bytes · \(prefix)" + (length > 16 ? " …" : "")
                    }
                    guard let bytes = sqlite3_column_text(statement, index) else { return "" }
                    let length = Int(sqlite3_column_bytes(statement, index))
                    return String(decoding: UnsafeBufferPointer(start: bytes, count: min(length, textLimit)), as: UTF8.self) + (length > textLimit ? "…" : "")
                })
            }
            return (columns, result)
        }
    }
    private func controls() {
        picker.isEnabled = !busy && !tables.isEmpty
        structure.isEnabled = !busy && !tables.isEmpty
        schemaButton.isEnabled = !busy && !tables.isEmpty
        previous.isEnabled = !busy && offset > 0 && structure.state == .off
        next.isEnabled = !busy && more && structure.state == .off
    }
    private func fail(_ message: String) {
        DispatchQueue.main.async { [weak self] in self?.busy = false; self?.status.stringValue = message; self?.controls() }
    }
    @objc func selectTable() { offset = 0; loadPage() }
    @objc func previousPage() { offset = max(0, offset - pageSize); loadPage() }
    @objc func nextPage() { offset += pageSize; loadPage() }
    private func loadPage() {
        guard !busy, tables.indices.contains(picker.indexOfSelectedItem) else { return }
        let name = tables[picker.indexOfSelectedItem].name.replacingOccurrences(of: "\"", with: "\"\"")
        let schema = structure.state == .on
        let queryText = schema ? "PRAGMA table_xinfo(\"\(name)\")" : "SELECT * FROM \"\(name)\" LIMIT \(pageSize + 1) OFFSET \(offset)"
        busy = true; status.stringValue = "Loading…"; controls()
        queue.async { [self] in
            do {
                let result = try query(queryText, limit: schema ? 256 : pageSize + 1)
                DispatchQueue.main.async { [weak self] in
                    guard let self = self else { return }
                    self.more = !schema && result.rows.count > self.pageSize
                    self.rows = Array(result.rows.prefix(schema ? 256 : self.pageSize))
                    for column in self.grid.tableColumns { self.grid.removeTableColumn(column) }
                    for (index, name) in result.columns.enumerated() {
                        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(String(index))); column.title = name; column.width = 180; self.grid.addTableColumn(column)
                    }
                    self.grid.reloadData(); self.busy = false; self.controls()
                    self.status.stringValue = schema ? "Table structure" : "\(self.rows.count) rows · offset \(self.offset) · up to 64 columns · long values shown as summaries"
                }
            } catch { fail(error.localizedDescription) }
        }
    }
    @objc func showSQL() {
        guard tables.indices.contains(picker.indexOfSelectedItem) else { return }
        let alert = NSAlert(); alert.messageText = "Table / View Definition"
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 560, height: 240)); scroll.hasVerticalScroller = true
        let text = NSTextView(frame: scroll.bounds); text.isEditable = false; text.font = .monospacedSystemFont(ofSize: 12, weight: .regular); text.string = tables[picker.indexOfSelectedItem].sql
        scroll.documentView = text; alert.accessoryView = scroll; alert.addButton(withTitle: "Close"); alert.runModal()
    }
    func numberOfRows(in tableView: NSTableView) -> Int { rows.count }
    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard let id = tableColumn?.identifier.rawValue, let column = Int(id), rows.indices.contains(row), rows[row].indices.contains(column) else { return nil }
        let label = NSTextField(labelWithString: rows[row][column]); label.lineBreakMode = .byTruncatingTail; label.isSelectable = true; label.toolTip = rows[row][column]; return label
    }
}
