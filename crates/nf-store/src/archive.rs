use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::StoreError;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// Shard file path: `{base}/{hash[0..2]}/{hash[2..4]}/{hash}`.
/// This keeps individual directories from growing too large.
fn sharded_path(base: &Path, hash: &str) -> PathBuf {
    base.join(&hash[..2]).join(&hash[2..4]).join(hash)
}

/// Merkle node metadata stored alongside data files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// SHA-256 hex hash of this node (covers children hashes + own data).
    pub hash: String,
    /// Ordered list of child node hashes.
    pub children: Vec<String>,
    /// Whether this node carries a data blob.
    pub has_data: bool,
}

// ─── DocumentArchive ─────────────────────────────────────────────────────────

/// Content-addressable document store backed by the local filesystem.
///
/// Files are stored by their SHA-256 hash in a sharded directory tree:
/// ```text
/// {base_path}/{hash[0..2]}/{hash[2..4]}/{hash}
/// ```
/// Merkle node metadata lives in a parallel `merkle/` subdirectory.
pub struct DocumentArchive {
    pub base_path: PathBuf,
}

impl DocumentArchive {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    // ── Document operations ──────────────────────────────────────────────────

    /// Store a document. Returns the SHA-256 hex hash (content address).
    /// Idempotent: calling again with the same data is a no-op.
    pub fn store(&self, data: &[u8]) -> Result<String, StoreError> {
        let hash = sha256_hex(data);
        let path = sharded_path(&self.base_path, &hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            fs::write(&path, data)?;
        }
        Ok(hash)
    }

    /// Retrieve a document by its SHA-256 hash.
    pub fn retrieve(&self, hash: &str) -> Result<Vec<u8>, StoreError> {
        let path = sharded_path(&self.base_path, hash);
        if !path.exists() {
            return Err(StoreError::NotFound(format!(
                "Document with hash {hash} not found"
            )));
        }
        Ok(fs::read(path)?)
    }

    /// Verify that a stored document still matches its declared hash.
    /// Returns `true` if integrity is confirmed, `false` if tampered.
    pub fn verify(&self, hash: &str) -> Result<bool, StoreError> {
        let data = self.retrieve(hash)?;
        let computed = sha256_hex(&data);
        Ok(computed == hash)
    }

    /// Check whether a hash exists in the store without loading the data.
    pub fn exists(&self, hash: &str) -> bool {
        sharded_path(&self.base_path, hash).exists()
    }

    // ── Merkle DAG ───────────────────────────────────────────────────────────

    /// Build a Merkle DAG node, optionally carrying a data blob.
    ///
    /// Node hash = BLAKE3(sorted_child_hashes || own_data).
    /// Using BLAKE3 for internal nodes gives the benefits of its built-in
    /// Merkle tree design while SHA-256 provides compatibility for leaf blobs.
    pub fn store_node(
        &self,
        children: &[String],
        data: Option<&[u8]>,
    ) -> Result<MerkleNode, StoreError> {
        // Sorted children → deterministic hash regardless of insertion order.
        let mut sorted_children = children.to_vec();
        sorted_children.sort();

        // Compute node hash with BLAKE3.
        let mut hasher = blake3::Hasher::new();
        for child in &sorted_children {
            hasher.update(child.as_bytes());
        }
        if let Some(d) = data {
            hasher.update(d);
        }
        let node_hash = hasher.finalize().to_hex().to_string();

        let node = MerkleNode {
            hash: node_hash.clone(),
            children: sorted_children,
            has_data: data.is_some(),
        };

        // Persist metadata.
        let meta_path = self.merkle_meta_path(&node_hash);
        if let Some(parent) = meta_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&meta_path, serde_json::to_vec(&node)?)?;

        // Persist data blob alongside the metadata.
        if let Some(d) = data {
            let data_path = self.merkle_data_path(&node_hash);
            if let Some(parent) = data_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if !data_path.exists() {
                fs::write(&data_path, d)?;
            }
        }

        Ok(node)
    }

    /// Load a Merkle node by its hash.
    pub fn get_node(&self, hash: &str) -> Result<MerkleNode, StoreError> {
        let meta_path = self.merkle_meta_path(hash);
        if !meta_path.exists() {
            return Err(StoreError::NotFound(format!(
                "Merkle node {hash} not found"
            )));
        }
        let bytes = fs::read(meta_path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Recursively verify a Merkle DAG rooted at `root_hash`.
    /// Returns `true` iff every node's hash is consistent with its contents.
    pub fn verify_dag(&self, root_hash: &str) -> Result<bool, StoreError> {
        self.verify_node_recursive(root_hash)
    }

    /// Walk every stored document file and verify its SHA-256 hash matches its
    /// filename (the content address).
    ///
    /// Returns a `Vec` of `PathBuf`s for every file whose hash did not match.
    /// An empty `Vec` means the archive is fully intact.
    ///
    /// The `merkle/` subdirectory is excluded; use [`Self::verify_dag`] for
    /// Merkle-node integrity.
    pub fn integrity_check(&self) -> Result<Vec<PathBuf>, StoreError> {
        let mut mismatches = Vec::new();

        // Walk the top-level two-character shard directories.
        let top_entries = match fs::read_dir(&self.base_path) {
            Ok(iter) => iter,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(mismatches),
            Err(e) => return Err(StoreError::Io(e)),
        };

        for top_entry in top_entries {
            let top_entry = top_entry?;
            let top_name = top_entry.file_name();
            let top_str = top_name.to_string_lossy();

            // Skip the merkle/ directory.
            if top_str == "merkle" {
                continue;
            }

            // Each entry under base_path should be a 2-char hex shard dir.
            if !top_entry.file_type()?.is_dir() {
                continue;
            }

            // Walk the second-level shard directories.
            for mid_entry in fs::read_dir(top_entry.path())? {
                let mid_entry = mid_entry?;
                if !mid_entry.file_type()?.is_dir() {
                    continue;
                }

                // Walk the actual data files.
                for file_entry in fs::read_dir(mid_entry.path())? {
                    let file_entry = file_entry?;
                    if !file_entry.file_type()?.is_file() {
                        continue;
                    }

                    let file_path = file_entry.path();
                    let file_name = file_entry.file_name();
                    let expected_hash = file_name.to_string_lossy();

                    let data = fs::read(&file_path)?;
                    let actual_hash = sha256_hex(&data);

                    if actual_hash != expected_hash.as_ref() {
                        mismatches.push(file_path);
                    }
                }
            }
        }

        Ok(mismatches)
    }

    fn merkle_meta_path(&self, hash: &str) -> PathBuf {
        self.base_path
            .join("merkle")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(format!("{hash}.meta"))
    }

    fn merkle_data_path(&self, hash: &str) -> PathBuf {
        self.base_path
            .join("merkle")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(hash)
    }

    fn verify_node_recursive(&self, hash: &str) -> Result<bool, StoreError> {
        let node = self.get_node(hash)?;

        // Recompute hash.
        let mut hasher = blake3::Hasher::new();
        for child in &node.children {
            hasher.update(child.as_bytes());
        }
        if node.has_data {
            let data_path = self.merkle_data_path(hash);
            if data_path.exists() {
                hasher.update(&fs::read(data_path)?);
            }
        }
        let computed = hasher.finalize().to_hex().to_string();
        if computed != hash {
            return Ok(false);
        }

        // Recurse into children.
        for child in &node.children {
            if !self.verify_node_recursive(child)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_archive() -> (TempDir, DocumentArchive) {
        let dir = TempDir::new().unwrap();
        let archive = DocumentArchive::new(dir.path().to_path_buf());
        (dir, archive)
    }

    #[test]
    fn test_store_and_retrieve() {
        let (_dir, archive) = tmp_archive();
        let data = b"Hello, nonfaction!";
        let hash = archive.store(data).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        let retrieved = archive.retrieve(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_store_idempotent() {
        let (_dir, archive) = tmp_archive();
        let data = b"idempotent";
        let h1 = archive.store(data).unwrap();
        let h2 = archive.store(data).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_retrieve_not_found() {
        let (_dir, archive) = tmp_archive();
        let fake_hash = "a".repeat(64);
        let result = archive.retrieve(&fake_hash);
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[test]
    fn test_verify_integrity() {
        let (_dir, archive) = tmp_archive();
        let data = b"verify me";
        let hash = archive.store(data).unwrap();
        assert!(archive.verify(&hash).unwrap());
    }

    #[test]
    fn test_verify_tampered_file() {
        let (_dir, archive) = tmp_archive();
        let data = b"original";
        let hash = archive.store(data).unwrap();

        // Tamper with the file on disk.
        let path = sharded_path(&archive.base_path, &hash);
        fs::write(&path, b"tampered!").unwrap();

        assert!(!archive.verify(&hash).unwrap());
    }

    #[test]
    fn test_exists() {
        let (_dir, archive) = tmp_archive();
        let data = b"exists?";
        let hash = archive.store(data).unwrap();
        assert!(archive.exists(&hash));
        assert!(!archive.exists(&"b".repeat(64)));
    }

    #[test]
    fn test_merkle_leaf_node() {
        let (_dir, archive) = tmp_archive();
        let node = archive.store_node(&[], Some(b"leaf data")).unwrap();
        assert!(!node.hash.is_empty());
        assert!(node.has_data);
        assert!(node.children.is_empty());

        // Verify DAG from this root.
        assert!(archive.verify_dag(&node.hash).unwrap());
    }

    #[test]
    fn test_merkle_tree() {
        let (_dir, archive) = tmp_archive();

        // Two leaves.
        let left = archive.store_node(&[], Some(b"left")).unwrap();
        let right = archive.store_node(&[], Some(b"right")).unwrap();

        // Parent node references both children.
        let parent = archive
            .store_node(&[left.hash.clone(), right.hash.clone()], None)
            .unwrap();

        // Verify the whole DAG.
        assert!(archive.verify_dag(&parent.hash).unwrap());
    }

    #[test]
    fn test_merkle_children_order_deterministic() {
        let (_dir, archive) = tmp_archive();
        let a = archive.store_node(&[], Some(b"a")).unwrap();
        let b = archive.store_node(&[], Some(b"b")).unwrap();

        let p1 = archive
            .store_node(&[a.hash.clone(), b.hash.clone()], None)
            .unwrap();
        let p2 = archive
            .store_node(&[b.hash.clone(), a.hash.clone()], None)
            .unwrap();

        // Sorted children → same parent hash regardless of argument order.
        assert_eq!(p1.hash, p2.hash);
    }

    #[test]
    fn test_get_node() {
        let (_dir, archive) = tmp_archive();
        let leaf = archive.store_node(&[], Some(b"data")).unwrap();
        let retrieved = archive.get_node(&leaf.hash).unwrap();
        assert_eq!(retrieved.hash, leaf.hash);
        assert_eq!(retrieved.has_data, leaf.has_data);
    }

    #[test]
    fn test_get_node_not_found() {
        let (_dir, archive) = tmp_archive();
        let fake = "f".repeat(64);
        assert!(matches!(
            archive.get_node(&fake),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn test_merkle_node_round_trip_metadata() {
        let (_dir, archive) = tmp_archive();
        let leaf = archive.store_node(&[], Some(b"payload")).unwrap();
        let loaded = archive.get_node(&leaf.hash).unwrap();

        assert_eq!(loaded.hash, leaf.hash);
        assert_eq!(loaded.children, leaf.children);
        assert_eq!(loaded.has_data, leaf.has_data);
    }

    #[test]
    fn test_merkle_internal_node_without_data() {
        let (_dir, archive) = tmp_archive();
        let left = archive.store_node(&[], Some(b"left-data")).unwrap();
        let right = archive.store_node(&[], Some(b"right-data")).unwrap();
        let root = archive
            .store_node(&[left.hash.clone(), right.hash.clone()], None)
            .unwrap();

        assert!(!root.has_data);
        assert_eq!(root.children.len(), 2);
        assert!(archive.verify_dag(&root.hash).unwrap());
    }

    #[test]
    fn test_merkle_verify_dag_detects_tampered_data_blob() {
        let (_dir, archive) = tmp_archive();
        let leaf = archive.store_node(&[], Some(b"clean-data")).unwrap();

        let data_path = archive.merkle_data_path(&leaf.hash);
        fs::write(data_path, b"tampered-data").unwrap();

        assert!(!archive.verify_dag(&leaf.hash).unwrap());
    }

    // ── integrity_check tests ────────────────────────────────────────────────

    #[test]
    fn test_integrity_check_empty_archive_returns_no_mismatches() {
        let (_dir, archive) = tmp_archive();
        let mismatches = archive.integrity_check().unwrap();
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_integrity_check_valid_files_returns_no_mismatches() {
        let (_dir, archive) = tmp_archive();
        archive.store(b"file one").unwrap();
        archive.store(b"file two").unwrap();
        archive.store(b"file three").unwrap();
        let mismatches = archive.integrity_check().unwrap();
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_integrity_check_detects_single_tampered_file() {
        let (_dir, archive) = tmp_archive();
        let hash = archive.store(b"genuine content").unwrap();
        let path = sharded_path(&archive.base_path, &hash);
        fs::write(&path, b"corrupted content").unwrap();
        let mismatches = archive.integrity_check().unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0], path);
    }

    #[test]
    fn test_integrity_check_detects_multiple_tampered_files() {
        let (_dir, archive) = tmp_archive();
        let h1 = archive.store(b"alpha").unwrap();
        let h2 = archive.store(b"beta").unwrap();
        archive.store(b"gamma").unwrap(); // not tampered
        let p1 = sharded_path(&archive.base_path, &h1);
        let p2 = sharded_path(&archive.base_path, &h2);
        fs::write(&p1, b"TAMPERED alpha").unwrap();
        fs::write(&p2, b"TAMPERED beta").unwrap();
        let mismatches = archive.integrity_check().unwrap();
        assert_eq!(mismatches.len(), 2);
        assert!(mismatches.contains(&p1));
        assert!(mismatches.contains(&p2));
    }

    #[test]
    fn test_integrity_check_ignores_merkle_directory() {
        let (_dir, archive) = tmp_archive();
        // Store a merkle node; it lives under base/merkle/...
        let _node = archive.store_node(&[], Some(b"merkle-payload")).unwrap();
        // The merkle sub-tree should be ignored by integrity_check.
        let mismatches = archive.integrity_check().unwrap();
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_integrity_check_reports_paths_under_base() {
        let (_dir, archive) = tmp_archive();
        let hash = archive.store(b"path check content").unwrap();
        let expected_path = sharded_path(&archive.base_path, &hash);
        // Tamper it.
        fs::write(&expected_path, b"corrupted").unwrap();
        let mismatches = archive.integrity_check().unwrap();
        assert!(mismatches[0].starts_with(&archive.base_path));
    }

    // ── IntegrityViolation error variant test ────────────────────────────────

    #[test]
    fn test_integrity_violation_error_variant_formats_message() {
        use crate::error::StoreError;
        let err = StoreError::IntegrityViolation("hash mismatch on file X".to_string());
        let msg = err.to_string();
        assert!(msg.contains("hash mismatch on file X"));
        assert!(msg.contains("Archive integrity violation"));
    }
}
