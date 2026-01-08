use anyhow::{Context, Result};
use git2::{Repository, BranchType};
use std::path::PathBuf;
use uuid::Uuid;

pub struct GitManager {
    repo: Repository,
    repo_path: PathBuf,
}

impl GitManager {
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let repo = Repository::discover(&current_dir)
            .context("Not in a Git repository")?;
        
        let repo_path = repo.workdir()
            .context("Repository has no working directory")?
            .to_path_buf();

        Ok(Self { repo, repo_path })
    }

    pub fn create_worktree(&self, name: &str) -> Result<PathBuf> {
        let branch_name = format!("shard_{}", Uuid::new_v4().simple());
        let worktree_path = self.repo_path.join(".shards").join(name);

        // Create .shards directory if it doesn't exist
        std::fs::create_dir_all(worktree_path.parent().unwrap())?;

        // Create new branch from HEAD
        let head = self.repo.head()?;
        let target_commit = head.peel_to_commit()?;
        
        self.repo.branch(&branch_name, &target_commit, false)
            .context("Failed to create branch")?;

        // Create worktree
        let opts = git2::WorktreeAddOptions::new();
        let _worktree = self.repo.worktree(
            &name,
            &worktree_path,
            Some(&opts)
        ).context("Failed to create worktree")?;

        // Checkout the new branch in the worktree
        let worktree_repo = Repository::open(&worktree_path)?;
        let branch = worktree_repo.find_branch(&branch_name, BranchType::Local)?;
        let branch_ref = branch.get();
        worktree_repo.set_head(branch_ref.name().unwrap())?;
        worktree_repo.checkout_head(None)?;

        Ok(worktree_path)
    }

    pub fn cleanup_worktree(&self, name: &str) -> Result<()> {
        // First, remove the worktree directory
        let worktree_path = self.repo_path.join(".shards").join(name);
        if worktree_path.exists() {
            std::fs::remove_dir_all(&worktree_path)?;
        }

        // Then prune the worktree reference
        if let Ok(worktree) = self.repo.find_worktree(name) {
            worktree.prune(None)?;
        }

        Ok(())
    }
}
