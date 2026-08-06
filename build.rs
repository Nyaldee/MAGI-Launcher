//! Embarque Icon.ico comme ressource Windows de l'exe (icône visible dans
//! l'Explorateur/la barre des tâches) -- build-only, voir Cargo.toml.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("Icon.ico")
            // Métadonnées de version Windows -- un exécutable non signé sans
            // aucune métadonnée (nom d'origine, description...) est un des
            // signaux que certains moteurs heuristiques/ML utilisent pour
            // juger un binaire "suspect", en plus de sa faible diffusion.
            .set("FileDescription", "MAGI Launcher")
            .set("ProductName", "MAGI Launcher")
            .set("OriginalFilename", "magi_launcher.exe")
            .set("InternalName", "magi_launcher")
            .set("CompanyName", "Nyaldee")
            .set("LegalCopyright", "Copyright © 2026 Nyaldee")
            .compile()
            .expect("échec de l'embarquement de l'icône");
    }
}
