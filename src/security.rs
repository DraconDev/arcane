use crate::auto_gitignore::AutoGitIgnore;
use crate::config::ArcaneConfig;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use rand::RngCore;
use regex::Regex;
use secrecy::ExposeSecret;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, ZeroizeOnDrop};

const REPO_KEY_LEN: usize = 32;

pub struct SecretScanner {
    patterns: Vec<(String, Regex)>,
}

impl SecretScanner {
    pub fn new() -> Self {
        let mut patterns = Vec::new();
        // AWS Access Key ID
        patterns.push((
            "AWS Access Key".to_string(),
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        ));
        // Stripe Live Key
        patterns.push((
            "Stripe Live Key".to_string(),
            Regex::new(r"sk_live_[0-9a-zA-Z]{24}").unwrap(),
        ));
        // Generic Private Key
        patterns.push((
            "Private Key".to_string(),
            Regex::new(r"-----BEGIN [A-Z ]+ PRIVATE KEY-----").unwrap(),
        ));
        // Google API Key
        patterns.push((
            "Google API Key".to_string(),
            Regex::new(r"AIza[0-9A-Za-z-_]{35}").unwrap(),
        ));

        Self { patterns }
    }

    pub fn scan(&self, content: &str) -> Vec<String> {
        let mut found = Vec::new();
        for (name, re) in &self.patterns {
            if re.is_match(content) {
                found.push(name.clone());
            }
        }
        found
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct RepoKey(Vec<u8>);

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct TeamKey(Vec<u8>);

impl RepoKey {
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        if bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid key length"));
        }
        Ok(RepoKey(bytes))
    }
}

pub struct ArcaneSecurity {
    master_identity: Option<x25519::Identity>,
    imported_identities: Vec<x25519::Identity>,
    #[allow(dead_code)]
    repo_keys: std::collections::HashMap<PathBuf, RepoKey>,
    scanner: SecretScanner,
    repo_root: Option<PathBuf>,
}

impl ArcaneSecurity {
    pub fn get_identity_path() -> PathBuf {
        let home = dirs::home_dir().expect("Could not find home directory");
        home.join(".arcane").join("identity.txt")
    }

    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let mut security = Self {
            master_identity: None,
            imported_identities: Vec::new(),
            repo_keys: std::collections::HashMap::new(),
            scanner: SecretScanner::new(),
            repo_root: repo_path.map(|p| p.to_path_buf()),
        };

        let idx = match security.load_master_identity() {
            Ok(id) => Some(id),
            Err(_) => None,
        };
        security.master_identity = idx;

        // Load imported legacy keys
        if let Ok(imported) = security.load_imported_identities() {
            if !imported.is_empty() {
                // println!("🔑 Loaded {} imported legacy identities", imported.len());
                security.imported_identities = imported;
            }
        }

        Ok(security)
    }

    /// Load generic identities from ~/.arcane/keys/*.age (e.g. Git Seal keys)
    fn load_imported_identities(&self) -> Result<Vec<x25519::Identity>> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let keys_dir = home.join(".arcane").join("keys");
        let mut identities = Vec::new();

        if !keys_dir.exists() {
            return Ok(identities);
        }

        use std::str::FromStr;
        for entry in std::fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(key_str) = content
                        .lines()
                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                    {
                        if let Ok(id) = x25519::Identity::from_str(key_str.trim()) {
                            identities.push(id);
                        }
                    }
                }
            }
        }
        Ok(identities)
    }

    /// Load the Master Identity from ~/.arcane/identity.age
    pub fn load_master_identity(&self) -> Result<x25519::Identity> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let identity_path = home.join(".arcane").join("identity.age");

        if !identity_path.exists() {
            return Err(anyhow::anyhow!("Identity file not found"));
        }

        let content = fs::read_to_string(&identity_path)?;
        // Assuming the file contains the Bech32 secret key string (AGE-SECRET-KEY-...)
        // potentially surrounded by whitespace or comments
        let key_str = content
            .lines()
            .find(|l| !l.starts_with('#') && !l.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("No key found in identity file"))?
            .trim();

        use std::str::FromStr;
        x25519::Identity::from_str(key_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse identity: {}", e))
    }

    pub fn has_master_identity(&self) -> bool {
        self.master_identity.is_some()
    }

    /// Explicitly generate and save a new Master Identity
    pub fn generate_master_identity(&mut self) -> Result<()> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let identity_path = home.join(".arcane").join("identity.age");

        if identity_path.exists() {
            return Err(anyhow::anyhow!("Identity already exists"));
        }

        // Generate new identity
        let key = x25519::Identity::generate();
        if let Some(parent) = identity_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&identity_path)?;
        writeln!(file, "{}", key.to_string().expose_secret())?;

        self.master_identity = Some(key);
        Ok(())
    }

    /// Helper to get the repo root, either from configured path or CWD
    fn get_repo_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.repo_root {
            return Ok(root.clone());
        }
        Self::find_repo_root()
    }

    /// Find the git repository root from the current directory
    pub fn find_repo_root() -> Result<PathBuf> {
        let mut current = std::env::current_dir()?;
        loop {
            if current.join(".git").exists() {
                return Ok(current);
            }
            if !current.pop() {
                return Err(anyhow::anyhow!("Not in a git repository"));
            }
        }
    }

    /// Load the repo key from .git/arcane/keys/*.age or history
    /// hierarchy:
    /// 1. Direct User Key: keys/<user>.age
    /// 2. Team Key: keys/team:<team>.age (decrypted via ~/.arcane/teams/<team>.key)
    /// 3. Machine Key: keys/machine:<hash>.age (decrypted via env var ARCANE_MACHINE_KEY)
    pub fn load_repo_key(&self) -> Result<RepoKey> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            // Fallback logic for legacy/uninit
            let legacy_path = repo_root.join(".git").join("arcane").join("repo.key");
            if legacy_path.exists() {
                return RepoKey::from_file(&legacy_path);
            }
            return Err(anyhow::anyhow!("No keys found. Run 'arcane init'."));
        }

        // 0. Try Machine Key (Env Var) - Priority for CI/CD
        if let Ok(machine_key_str) = std::env::var("ARCANE_MACHINE_KEY") {
            // Derive identity from the env var string
            use std::str::FromStr;
            if let Ok(machine_identity) = x25519::Identity::from_str(&machine_key_str) {
                if let Ok(key) = self.try_decrypt_directory_machine(&keys_dir, &machine_identity) {
                    return Ok(key);
                }
            }
        }

        // Check for Master Identity (Interactve User)
        let identity_opt = self.master_identity.as_ref();

        if let Some(identity) = identity_opt {
            // 1. Try direct User access (keys/*.age)
            if let Ok(key) = self.try_decrypt_directory(&keys_dir, identity) {
                return Ok(key);
            }

            // 1b. Try Imported Identities (Heritage Keys / Git Seal)
            for imported_id in &self.imported_identities {
                if let Ok(key) = self.try_decrypt_directory(&keys_dir, imported_id) {
                    // println!("🔓 Unlocked via imported identity");
                    return Ok(key);
                }
            }

            // 2. Try Team access (keys/team:*.age)
            for entry in fs::read_dir(&keys_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    if filename.starts_with("team:") && filename.ends_with(".age") {
                        let team_name = filename
                            .trim_start_matches("team:")
                            .trim_end_matches(".age");

                        if let Ok(team_key) = self.load_team_key(team_name) {
                            if let Ok(repo_key) =
                                self.decrypt_repo_key_with_team_key(&path, &team_key)
                            {
                                return Ok(repo_key);
                            }
                        }
                    }
                }
            }

            // 3. Try history keys (latest to oldest)
            let history_dir = keys_dir.join("history");
            if history_dir.exists() {
                let mut entries: Vec<_> =
                    fs::read_dir(history_dir)?.filter_map(|e| e.ok()).collect();
                entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
                for entry in entries {
                    if entry.path().is_dir() {
                        if let Ok(key) = self.try_decrypt_directory(&entry.path(), identity) {
                            return Ok(key);
                        }
                    }
                }
            }
        }

        // 4. Last Resort: Legacy repo.key
        let legacy_path = repo_root.join(".git").join("arcane").join("repo.key");
        if legacy_path.exists() {
            return RepoKey::from_file(&legacy_path);
        }

        Err(anyhow::anyhow!(
            "Access Denied. Missing valid Key (User, Team, or Machine)."
        ))
    }

    /// Authorize a new recipient (Machine or User) to access this repository
    pub fn authorize_recipient(&self, recipient: &age::x25519::Recipient) -> Result<()> {
        let repo_key = self.load_repo_key()?;
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        std::fs::create_dir_all(&keys_dir)?;

        let output_path = keys_dir.join(format!("{}.age", recipient));

        // Encrypt the repo key for the recipient
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
        let encryptor = age::Encryptor::with_recipients(
            recipients.iter().map(|b| b.as_ref() as &dyn age::Recipient),
        )
        .expect("Failed to create encryptor");

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        std::fs::write(&output_path, encrypted)?;
        Ok(())
    }

    // specialized helper for machine key scanning
    fn try_decrypt_directory_machine(
        &self,
        dir: &Path,
        identity: &x25519::Identity,
    ) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            // Machine keys are stored as machine:<hash>.age or just generic .age files?
            // "authorize_machine" will likely prefix them or just use the pubkey hash.
            // Let's just try to decrypt ALL .age files. The identity will fail if not a recipient.
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                // Optimization: Only try files that look like machine keys?
                // Or just try specific ones.
                // "machine:<hash>.age"
                if filename.starts_with("machine:") {
                    if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                        return Ok(repo_key);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("No matching machine key found"))
    }

    /// Generate a new Machine Identity (Private Key, Public Key)
    /// Returns strings suitable for display/env vars.
    pub fn generate_machine_identity() -> (String, String) {
        let identity = x25519::Identity::generate();
        let pub_key = identity.to_public().to_string();
        let identity_str = identity.to_string();
        let priv_key = identity_str.expose_secret();
        (priv_key.to_string(), pub_key)
    }

    /// Authorize a Machine (Public Key) to access this repo
    pub fn whitelist_machine(&self, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid machine public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to whitelist machines")?;

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        // Use hash or similar ID for filename
        // To keep it clean, maybe just first 16 chars of pubkey?
        // Pubkey is "age1..." (Bech32).
        let safe_name = public_key_str
            .replace(":", "_")
            .chars()
            .take(12)
            .collect::<String>();
        let machine_file = keys_dir.join(format!("machine:{}.age", safe_name));

        self.encrypt_and_save_key(&repo_key, &recipient, &machine_file)?;

        Ok(())
    }

    /// Load a Team Key from ~/.arcane/teams/<name>.key
    pub fn load_team_key(&self, team_name: &str) -> Result<TeamKey> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_key_path = home
            .join(".arcane")
            .join("teams")
            .join(format!("{}.key", team_name));

        if !team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team key '{}' not found in keychain",
                team_name
            ));
        }

        // Team keys are encrypted with Master Identity
        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity required to unlock team keys")?;

        // Decrypt the file
        let encrypted_bytes = fs::read(&team_key_path)?;
        let decryptor = age::Decryptor::new(&encrypted_bytes[..])?;
        let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;

        let mut key_bytes = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid team key length"));
        }

        Ok(TeamKey(key_bytes))
    }

    fn decrypt_repo_key_with_team_key(&self, path: &Path, team_key: &TeamKey) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;

        // Team Key is a symmetric key? Or treating it as an Identity?
        // Ideally Team Key is a symmetric key (ChaCha20-Poly1305) used to encrypt the Repo Key.
        // But age works best with Identities.
        // Option A: Use age with a passphrase (the team key bytes as passphrase).
        // Option B: Team Key *is* an age identity (x25519).

        // Decision: Team Key is an x25519 Identity.
        // When we "create a team", we generate a new x25519 Identity.
        // We store the PRIVATE key encrypted on the user's disk.
        // We use the Team's PUBLIC key to encrypt the Repo Key.

        let key_str = std::str::from_utf8(&team_key.0)?;
        // This implies team_key.0 contains the string representation of the identity?
        // Let's adjust TeamKey creation to store the raw Identity string or bytes.

        use std::str::FromStr;
        let team_identity = x25519::Identity::from_str(key_str)
            .map_err(|_| anyhow::anyhow!("Invalid team identity format"))?;

        let decryptor = age::Decryptor::new(&encrypted_bytes[..])?;
        let mut reader =
            decryptor.decrypt(std::iter::once(&team_identity as &dyn age::Identity))?;

        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }

        Ok(RepoKey(key_bytes))
    }

    /// Create a new Team (Generates a new Identity, saves to keychain)
    pub fn create_team(&self, team_name: &str) -> Result<()> {
        // Validate name
        if team_name.contains('/') || team_name.contains('\\') || team_name.contains(':') {
            return Err(anyhow::anyhow!("Invalid team name"));
        }

        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".arcane").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));
        if team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team '{}' already exists in your keychain",
                team_name
            ));
        }

        // Generate new Identity for the team
        let team_identity = x25519::Identity::generate();
        let team_identity_string = team_identity.to_string(); // Extend lifetime
        let team_secret = team_identity_string.expose_secret();

        // Encrypt this secret with Master Identity for storage
        let master = self
            .master_identity
            .as_ref()
            .context("Master identity required")?;
        let recipient = master.to_public();

        // Encryption logic
        let recipients = vec![&recipient as &dyn age::Recipient];
        let encryptor = age::Encryptor::with_recipients(recipients.into_iter())?;

        let mut file = fs::File::create(&team_key_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(team_secret.as_bytes())?;
        writer.finish()?;

        Ok(())
    }

    /// Allow the Team to unlock this repo (Encrypt RepoKey with Team Public Key)
    pub fn add_repo_to_team(&self, team_name: &str) -> Result<()> {
        let team_key = self.load_team_key(team_name)?;

        // Reconstruct Identity to get Public Key
        let key_str = std::str::from_utf8(&team_key.0)?;
        use std::str::FromStr;
        let team_identity = x25519::Identity::from_str(key_str)
            .map_err(|_| anyhow::anyhow!("Invalid team identity"))?;
        let team_recipient = team_identity.to_public();

        let repo_key = self.load_repo_key()?;

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        let team_file_path = keys_dir.join(format!("team:{}.age", team_name));

        self.encrypt_and_save_key(&repo_key, &team_recipient, &team_file_path)?;

        Ok(())
    }

    fn try_decrypt_directory(&self, dir: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                    return Ok(repo_key);
                }
            }
        }
        Err(anyhow::anyhow!("No decryptable key in directory"))
    }

    /// Rotate the repository encryption key
    ///
    /// 1. Moves current keys to keys/history/<timestamp>/
    /// 2. Generates new key
    /// 3. Encrypts new key for all 'kept' members (using .pub files)
    ///
    pub fn rotate_repo_key(&self, keep_aliases: &[String]) -> Result<()> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        let history_dir = keys_dir.join("history");

        // 1. Create history timestamp dir
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .to_string();
        let backup_path = history_dir.join(&timestamp);
        fs::create_dir_all(&backup_path)?;

        // 2. Move existing .age files to history
        // Note: We copy .pub files too? Or leave them? We leave them for re-encryption.
        // Actually, let's copy everything to history to be safe state snapshot,
        // then delete .age files from keys_dir.
        for entry in fs::read_dir(&keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                // Move .age, Copy .pub?
                // Simpler: Move everything that is a key file.
                let name = path.file_name().unwrap();
                fs::copy(&path, backup_path.join(name))?;

                // Remove old .age files from current dir
                if path.extension().map_or(false, |e| e == "age") {
                    fs::remove_file(&path)?;
                }
            }
        }

        // 3. Generate New Key
        let new_repo_key = self.generate_repo_key()?;

        // 4. Encrypt for kept members
        for alias in keep_aliases {
            let pub_path = keys_dir.join(format!("{}.pub", alias));
            if !pub_path.exists() {
                eprintln!(
                    "⚠️ Warning: No public key found for '{}', skipping (they will lose access)",
                    alias
                );
                continue;
            }

            let pub_key_str = fs::read_to_string(&pub_path)?;
            let recipient: x25519::Recipient = pub_key_str
                .trim()
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid stored public key for {}: {}", alias, e))?;

            let key_path = keys_dir.join(format!("{}.age", alias));
            self.encrypt_and_save_key(&new_repo_key, &recipient, &key_path)?;
        }

        Ok(())
    }

    fn try_decrypt_key_file(&self, path: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;
        let decryptor = age::Decryptor::new(&encrypted_bytes[..])?;

        // Decryptor is a struct in 0.11+, handles recipients internally
        let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;

        let mut key_bytes = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }

        Ok(RepoKey(key_bytes))
    }

    /// Import an existing raw 32-byte key (e.g. from Git Seal)
    pub fn import_repo_key(&self, key_bytes: &[u8]) -> Result<PathBuf> {
        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!(
                "Invalid key length: expected 32 bytes, got {}",
                key_bytes.len()
            ));
        }

        let repo_root = self.get_repo_root()?;
        let arcane_dir = repo_root.join(".git").join("arcane");
        let keys_dir = arcane_dir.join("keys");

        if keys_dir.exists() {
            // stricter check? or allow overwrite?
            // For import, we probably want to fail if already initialized to avoid accidental overwrite
            if keys_dir.read_dir()?.next().is_some() {
                return Err(anyhow::anyhow!(
                    "Repo already initialized. Remove keys to import."
                ));
            }
        }
        fs::create_dir_all(&keys_dir)?;

        let repo_key = RepoKey(key_bytes.to_vec());

        // Get own identity to encrypt for self
        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity needed to import")?;
        let recipient = identity.to_public();

        // Save owner's public key
        let pub_key_path = keys_dir.join("owner.pub");
        fs::write(&pub_key_path, recipient.to_string())?;

        // Encrypt and save as 'owner.age'
        let key_path = keys_dir.join("owner.age");
        self.encrypt_and_save_key(&repo_key, &recipient, &key_path)?;

        // Auto-configure Git filters
        self.configure_git_filters(&repo_root)?;

        Ok(key_path)
    }

    /// Initialize security for the current repo (generate key and encrypt for self)
    pub fn init_repo(&self) -> Result<PathBuf> {
        let repo_root = self.get_repo_root()?;
        let arcane_dir = repo_root.join(".git").join("arcane");
        let keys_dir = arcane_dir.join("keys");

        if keys_dir.exists() {
            // If directory exists but is empty, allow re-initialization
            if keys_dir.read_dir()?.next().is_some() {
                return Err(anyhow::anyhow!(
                    "Repo already initialized at {:?}",
                    keys_dir
                ));
            }
        }

        fs::create_dir_all(&keys_dir)?;

        // Generate new repo key
        let repo_key = self.generate_repo_key()?;

        // Get own identity to encrypt for self
        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity needed to init")?;
        let recipient = identity.to_public();

        // Save owner's public key for rotation
        let pub_key_path = keys_dir.join("owner.pub");
        fs::write(&pub_key_path, recipient.to_string())?;

        // Encrypt and save as 'owner.age' (or derived name)
        // Using 'owner.age' for the initial key
        let key_path = keys_dir.join("owner.age");
        self.encrypt_and_save_key(&repo_key, &recipient, &key_path)?;

        // Auto-configure Git filters
        self.configure_git_filters(&repo_root)?;

        Ok(key_path)
    }

    fn encrypt_and_save_key(
        &self,
        repo_key: &RepoKey,
        recipient: &x25519::Recipient,
        path: &Path,
    ) -> Result<()> {
        let recipients = vec![recipient as &dyn age::Recipient];
        let encryptor = age::Encryptor::with_recipients(recipients.into_iter())
            .context("Failed to create encryptor")?;

        let mut file = fs::File::create(path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        Ok(())
    }

    /// Add a new team member by encrypting the repo key for them
    pub fn add_team_member(&self, alias: &str, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to add members")?;

        // Sanitize alias
        let alias = alias.trim();
        if alias.is_empty() || alias.contains('/') || alias.contains('\\') {
            return Err(anyhow::anyhow!("Invalid alias"));
        }

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        let key_path = keys_dir.join(format!("{}.age", alias));
        let pub_key_path = keys_dir.join(format!("{}.pub", alias));

        if key_path.exists() {
            return Err(anyhow::anyhow!("Member '{}' already exists", alias));
        }

        // Save public key
        fs::write(&pub_key_path, public_key_str)?;

        // Save Age key
        self.encrypt_and_save_key(&repo_key, &recipient, &key_path)?;

        Ok(())
    }

    /// Create an Invite for a user to join a Team
    /// Result: arcane/invites/<team>/<invite_file>.age
    pub fn create_team_invite(&self, team_name: &str, user_public_key: &str) -> Result<()> {
        let team_key = self.load_team_key(team_name)?;

        let recipient: x25519::Recipient = user_public_key
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid user public key: {}", e))?;

        let repo_root = self.get_repo_root()?;

        // Use hash of pubkey for filename to avoid leakage/collisions
        // Or just use a random ID?
        // Let's use a safe-to-filename version of the pubkey (or first 8 chars)
        // age1... -> age1...
        // But better is to just generate a random invite ID.
        let invite_id = uuid::Uuid::new_v4().to_string();

        let invites_dir = repo_root.join("arcane").join("invites").join(team_name);
        fs::create_dir_all(&invites_dir)?;

        let invite_path = invites_dir.join(format!("{}.age", invite_id));

        // Encrypt the TEAM KEY (bytes) for the USER
        // We need to re-wrap TeamKey as a "RepoKey" structure just for the helper?
        // Or just write bytes using encryptor manually.

        let recipients = vec![&recipient as &dyn age::Recipient];
        let encryptor = age::Encryptor::with_recipients(recipients.into_iter())?;

        let mut file = fs::File::create(&invite_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&team_key.0)?;
        writer.finish()?;

        Ok(())
    }

    /// Accept a Team Invite
    /// Reads arbitrary invite file, decrypts it (expecting a Team Key), saves to keychain.
    pub fn accept_team_invite(&self, invite_path: &Path) -> Result<String> {
        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity required to accept invite")?;

        let encrypted_bytes = fs::read(invite_path)?;
        let decryptor = age::Decryptor::new(&encrypted_bytes[..])?;
        let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;

        let mut key_bytes = Vec::new();
        use std::io::Read; // Fix for E0599
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid invite content"));
        }

        // We have the raw team key. But we don't know the Team Name from the file content (bytes only).
        // The user must provide the name, or we derive it from the invite path?
        // Invite path: arcane/invites/<team_name>/...
        let team_name = invite_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine team name from path"))?;

        // Save to key chain
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".arcane").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));

        if team_key_path.exists() {
            // Overwrite? Warn?
            // Checking if same key?
            // For now, allow overwrite.
        }

        // Encrypt for local storage (Master Identity)
        let master = self.master_identity.as_ref().unwrap();
        let recipient = master.to_public();

        let recipients = vec![&recipient as &dyn age::Recipient];
        let encryptor = age::Encryptor::with_recipients(recipients.into_iter())?;

        let mut file = fs::File::create(&team_key_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&key_bytes)?;
        writer.finish()?;

        Ok(team_name.to_string())
    }

    /// List all team members (aliases)
    pub fn list_team_members(&self) -> Result<Vec<String>> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return Ok(Vec::new());
        }

        let mut members = Vec::new();
        for entry in fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    members.push(stem.to_string());
                }
            }
        }

        Ok(members)
    }

    fn configure_git_filters(&self, repo_root: &Path) -> Result<()> {
        use std::process::Command;

        let exe_path = std::env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();

        // Ideally we use "arcane" if it's in PATH, but using absolute path is safer for filters
        // cleaning command
        Command::new("git")
            .current_dir(repo_root)
            .args(&[
                "config",
                "filter.git-arcane.clean",
                &format!("'{}' clean %f", exe_str),
            ])
            .output()
            .context("Failed to configure git-arcane.clean")?;

        // smudge command
        Command::new("git")
            .current_dir(repo_root)
            .args(&[
                "config",
                "filter.git-arcane.smudge",
                &format!("'{}' smudge", exe_str),
            ])
            .output()
            .context("Failed to configure git-arcane.smudge")?;

        // required
        Command::new("git")
            .current_dir(repo_root)
            .args(&["config", "filter.git-arcane.required", "true"])
            .output()
            .context("Failed to configure git-arcane.required")?;

        // 4. Update .gitattributes (Enforce Config Source of Truth)
        let attributes_path = repo_root.join(".gitattributes");
        let mut content = String::new();
        if attributes_path.exists() {
            content = fs::read_to_string(&attributes_path)?;
        }

        // Load Config
        let config = ArcaneConfig::load().unwrap_or_default();
        let desired_patterns = config.gitattributes_patterns;

        // Filter out existing lines that match our managed patterns or legacy filters
        // This effectively "Resets" the Arcane section while keeping user custom attributes
        let mut lines: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .filter(|l| {
                // Remove lines that explicitly use arcane filters
                if l.contains("filter=git-arcane") || l.contains("filter=git-seal") {
                    return false;
                }
                // Remove lines that match our desired patterns (to avoid duplication when we re-add)
                // logic: if 'l' is exactly one of our desired patterns
                if desired_patterns.contains(l) {
                    return false;
                }
                true
            })
            .collect();

        // Add Header
        lines.push("# Auto-committer gitattributes (Managed by Arcane)".to_string());

        // Add desired patterns
        for pattern in desired_patterns {
            lines.push(pattern);
        }

        // Write back
        let new_content = lines.join("\n");
        // Ensure trailing newline
        let final_content = if new_content.ends_with('\n') {
            new_content
        } else {
            format!("{}\n", new_content)
        };

        fs::write(&attributes_path, final_content)?;

        // 5. Enforce Tracking (Remove .env from .gitignore)
        let auto_ignore = AutoGitIgnore::new(repo_root);
        // We strictly want these files TRACKED (so they are encrypted), not ignored.
        // We remove any pattern that would ignore them.
        let _ = auto_ignore.ensure_tracked(&["*.env", ".env", ".env.*"]);

        Ok(())
    }

    /// Recursively scan the repository for secrets (respecting .gitignore)
    pub fn scan_repo(&self) -> Result<Vec<(PathBuf, Vec<String>)>> {
        let repo_root = self.get_repo_root()?;
        let mut findings = Vec::new();

        let walker = ignore::WalkBuilder::new(&repo_root)
            .hidden(false) // Don't skip hidden files automatically (we want .env)
            .git_ignore(true) // DO respect .gitignore (skip node_modules)
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        let path = entry.path();

                        // Explicitly skip .git/ directory content if it somehow leaks through
                        // (WalkBuilder usually handles this, but .git/arcane might be interesting? No, don't scan internal storage)
                        if path.components().any(|c| c.as_os_str() == ".git") {
                            continue;
                        }

                        // Try reading as text. If it fails (binary), we skip.
                        // We also likely want to skip the identity file itself if it's in the repo?
                        // But finding it would be good (it shouldn't be in repo).
                        if let Ok(content) = fs::read_to_string(path) {
                            let matches = self.scan_content(&content);
                            if !matches.is_empty() {
                                findings.push((path.to_path_buf(), matches));
                            }
                        }
                    }
                }
                Err(err) => eprintln!("Scan warning: {}", err),
            }
        }
        Ok(findings)
    }

    /// Scan content for secrets
    pub fn scan_content(&self, content: &str) -> Vec<String> {
        self.scanner.scan(content)
    }

    /// Git Clean Filter: Encrypt stdin -> stdout
    /// If file_path is provided and matches .env pattern, create a plaintext backup.
    pub fn seal_clean(&self, file_path: Option<&str>) -> Result<()> {
        use std::io::{Read, Write};

        // Auto-init if no key found (enables global .gitattributes config)
        let repo_key = match self.load_repo_key() {
            Ok(key) => key,
            Err(_) => {
                // Attempt auto-initialization
                eprintln!("🔧 No Repo Key found. Auto-initializing...");
                self.init_repo().context("Auto-init failed")?;
                eprintln!("✅ Repo initialized.");
                self.load_repo_key()
                    .context("Failed to load key after auto-init")?
            }
        };

        // 1. Read plaintext from stdin
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;

        // 2. Encrypt
        let encrypted = self.encrypt_with_repo_key(&repo_key, &buffer)?;

        // 3. Backup if necessary (Safety Net for .env files)
        if let Some(path) = file_path {
            if path.contains(".env") {
                // Ignore error on backup failure to not break git flow, but log it?
                // For now, allow it to fail seal_clean if backup fails (safe default)
                let _ = self.backup_secret(path, &buffer)?;
            }
        }

        // 4. Write ciphertext to stdout
        std::io::stdout().write_all(&encrypted)?;
        Ok(())
    }

    fn backup_secret(&self, original_path: &str, content: &[u8]) -> Result<()> {
        let repo_root = self.get_repo_root()?;
        let backup_dir = repo_root.join(".git").join("arcane").join("backups");
        fs::create_dir_all(&backup_dir)?;

        // Create safe filename (sanitize path separators)
        let safe_name = original_path.replace("/", "_").replace("\\", "_");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        // Use .age extension for encrypted backups
        let backup_path = backup_dir.join(format!("{}.{}.bak.age", safe_name, timestamp));

        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity required for secure backup")?;
        let recipient = identity.to_public();

        // Encrypt with Master Key
        let recipients = vec![&recipient as &dyn age::Recipient];
        let encryptor = age::Encryptor::with_recipients(recipients.into_iter())
            .context("Failed to create encryptor for backup")?;

        let mut file = fs::File::create(&backup_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(content)?;
        writer.finish()?;

        Ok(())
    }

    pub fn list_snapshots(&self) -> Result<Vec<(String, String, u64)>> {
        let repo_root = self.get_repo_root()?;
        let backup_dir = repo_root.join(".git").join("arcane").join("backups");

        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();
        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let metadata = fs::metadata(&path)?;
                // Attempt to parse original name and timestamp from "original_name.timestamp.bak.age"
                // Format: {safe_name}.{timestamp}.bak.age
                // We can return the raw filename, and the frontend can parse, or parse here.
                // Let's return (filename, original_path_guess, timestamp)

                // Split by dots from right
                // a.b.c.12345.bak.age
                // This is tricky if filename has dots.
                // Simple approach: Return raw filename and file modification time (or parsed timestamp if possible)
                snapshots.push((name, path.to_string_lossy().to_string(), metadata.len()));
            }
        }
        // Sort by time descending
        snapshots.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(snapshots)
    }

    pub fn restore_snapshot(&self, snapshot_filename: &str, target_path: &str) -> Result<()> {
        let repo_root = self.get_repo_root()?;
        let backup_path = repo_root
            .join(".git")
            .join("arcane")
            .join("backups")
            .join(snapshot_filename);

        if !backup_path.exists() {
            return Err(anyhow::anyhow!("Snapshot not found"));
        }

        let identity = self
            .master_identity
            .as_ref()
            .context("Master identity required to restore")?;

        // Decrypt
        let encrypted_bytes = fs::read(&backup_path)?;
        let decryptor = age::Decryptor::new(&encrypted_bytes[..])?;
        let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;

        let mut plaintext = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut plaintext)?;

        // Write to target
        // If target_path is relative, join with repo_root, else use as is (careful with absolute paths)
        // For security, probably enforce target is within repo.
        let target_full_path = repo_root.join(target_path); // rudimentary

        // Ensure parent dir exists
        if let Some(parent) = target_full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&target_full_path, plaintext)?;
        Ok(())
    }

    /// Git Smudge Filter: Decrypt stdin -> stdout
    pub fn seal_smudge(&self) -> Result<()> {
        use std::io::{Read, Write};

        let repo_key = self.load_repo_key()?;

        // 1. Read ciphertext from stdin
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;

        // 2. Decrypt
        let plaintext = self.decrypt_with_repo_key(&repo_key, &buffer)?;

        // 3. Write plaintext to stdout
        std::io::stdout().write_all(&plaintext)?;
        Ok(())
    }

    /// Generate a new symmetric key for a repository
    pub fn generate_repo_key(&self) -> Result<RepoKey> {
        let mut key_bytes = [0u8; REPO_KEY_LEN];
        rand::rng().fill_bytes(&mut key_bytes);
        Ok(RepoKey(key_bytes.to_vec()))
    }

    /// Encrypt data using the repo key (AES-GCM)
    pub fn encrypt_with_repo_key(&self, repo_key: &RepoKey, data: &[u8]) -> Result<Vec<u8>> {
        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits; unique per message

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failure: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypt data using the repo key
    pub fn decrypt_with_repo_key(
        &self,
        repo_key: &RepoKey,
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            // Graceful fallback: If data is too short, it might be plain text or empty.
            // For filter, error to be safe.
            return Err(anyhow::anyhow!("Invalid ciphertext length"));
        }

        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failure: {}", e))?;

        Ok(plaintext)
    }
}
