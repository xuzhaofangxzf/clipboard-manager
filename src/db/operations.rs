use super::schema::ClipboardEntry;
use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;

const ENTRIES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("clipboard_entries");
const COUNTER_TABLE: TableDefinition<&str, u64> = TableDefinition::new("counters");

/// Database manager for clipboard history
pub struct ClipboardDatabase {
    db: Arc<Database>,
}

impl ClipboardDatabase {
    /// Open or create database at the given path
    pub fn open(path: PathBuf) -> Result<Self> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(ENTRIES_TABLE)?;
            let _ = write_txn.open_table(COUNTER_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Insert a new clipboard entry
    pub fn insert_entry(&self, mut entry: ClipboardEntry) -> Result<u64> {
        let write_txn = self.db.begin_write()?;

        // Get next ID
        let id = {
            let mut counter_table = write_txn.open_table(COUNTER_TABLE)?;
            let current = counter_table
                .get("entry_id")?
                .map(|v| v.value())
                .unwrap_or(0);
            let next_id = current + 1;
            counter_table.insert("entry_id", next_id)?;
            next_id
        };

        // Set ID and insert entry
        entry.id = id;
        let bytes = entry.to_bytes()?;

        {
            let mut entries_table = write_txn.open_table(ENTRIES_TABLE)?;
            entries_table.insert(id, bytes.as_slice())?;
        }

        write_txn.commit()?;
        Ok(id)
    }

    /// Get entries with pagination (newest first)
    pub fn get_entries(&self, offset: usize, limit: usize) -> Result<Vec<ClipboardEntry>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ENTRIES_TABLE)?;

        let mut entries = Vec::new();
        let mut all_entries: Vec<_> = table.iter()?.collect::<Result<Vec<_>, _>>()?;

        // Sort by ID descending (newest first)
        all_entries.sort_by(|a, b| b.0.value().cmp(&a.0.value()));

        for (_, value) in all_entries.into_iter().skip(offset).take(limit) {
            if let Ok(entry) = ClipboardEntry::from_bytes(value.value()) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Get a specific entry by ID
    pub fn get_entry_by_id(&self, id: u64) -> Result<Option<ClipboardEntry>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ENTRIES_TABLE)?;

        if let Some(value) = table.get(id)? {
            Ok(Some(ClipboardEntry::from_bytes(value.value())?))
        } else {
            Ok(None)
        }
    }

    /// Delete an entry by ID
    pub fn delete_entry(&self, id: u64) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ENTRIES_TABLE)?;
            table.remove(id)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Get total count of entries
    // pub fn count_entries(&self) -> Result<usize> {
    //     let read_txn = self.db.begin_read()?;
    //     let table = read_txn.open_table(ENTRIES_TABLE)?;
    //     Ok(table.len()?)
    // }

    /// Clear old entries to maintain max count
    pub fn clear_old_entries(&self, max_count: usize) -> Result<()> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ENTRIES_TABLE)?;

        let mut all_ids: Vec<_> = table
            .iter()?
            .filter_map(|r| r.ok())
            .map(|(k, _)| k.value())
            .collect();

        drop(table);
        drop(read_txn);

        if all_ids.len() <= max_count {
            return Ok(());
        }

        // Sort by ID descending
        all_ids.sort_by(|a, b| b.cmp(a));

        // Delete entries beyond max_count
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ENTRIES_TABLE)?;
            for id in all_ids.into_iter().skip(max_count) {
                table.remove(id)?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Search entries by content
    pub fn search_entries(&self, query: &str, limit: usize) -> Result<Vec<ClipboardEntry>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ENTRIES_TABLE)?;

        let query_lower = query.to_lowercase();
        let mut entries = Vec::new();
        let mut all_entries: Vec<_> = table.iter()?.collect::<Result<Vec<_>, _>>()?;

        // Sort by ID descending
        all_entries.sort_by(|a, b| b.0.value().cmp(&a.0.value()));

        for (_, value) in all_entries {
            if entries.len() >= limit {
                break;
            }

            if let Ok(entry) = ClipboardEntry::from_bytes(value.value()) {
                if entry.preview.to_lowercase().contains(&query_lower) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }
}
