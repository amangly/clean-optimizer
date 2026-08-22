use crate::error::Result;
use std::fs;
use std::path::PathBuf;

pub struct Paths {
    pub root: PathBuf,
    pub user: PathBuf,
}

impl Paths {
    pub fn live() -> Result<Self> {
        let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
        let root = PathBuf::from(program_data).join("CleanOptimizer");
        let sid = current_sid();
        let user = root.join("users").join(sid);
        fs::create_dir_all(root.join("backup"))?;
        fs::create_dir_all(user.join("config"))?;
        fs::create_dir_all(user.join("profiles"))?;
        fs::create_dir_all(user.join("logs"))?;
        Ok(Self { root, user })
    }

}

fn current_sid() -> String {
    #[cfg(windows)]
    {
        crate::win::current_user_sid().unwrap_or_else(|_| "unknown".into())
    }
    #[cfg(not(windows))]
    {
        "unknown".into()
    }
}
