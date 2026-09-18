//! Embarque Icon.ico comme ressource Windows de l'exe (icône visible dans
//! l'Explorateur/la barre des tâches) -- build-only, voir Cargo.toml.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("Icon.ico")
            // Métadonnées de version Windows : un exécutable non signé et
            // dépourvu de toute métadonnée compte parmi les signaux qu'un
            // moteur antivirus heuristique retient pour le juger suspect.
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
