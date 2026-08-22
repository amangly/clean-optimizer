<p align="center">
  <img src="src/assets/logo.png" width="88" alt="Clean Optimizer" />
</p>

<h1 align="center">Clean Optimizer</h1>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri" />
  <img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black" alt="React" />
</p>

Windows app. Writes registry, power plan, and service settings for the Chinese Delta Force client (三角洲行动). Each write is backed up. Restore puts the old value back. The game install is left alone.

Looks for `DeltaForceClient-Win64-Shipping.exe`, `DeltaForceClient.exe`, or `DeltaForce.exe`. Other clients are not supported yet. System items need administrator.

```
npm install
npx tauri build
```

Installer: `src-tauri\target\release\bundle\nsis\`. The release exe has no console window.
