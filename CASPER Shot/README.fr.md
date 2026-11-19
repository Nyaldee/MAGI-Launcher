# CASPER Shot

*[Read in English](README.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Un petit outil de capture d'écran pour Windows : deux raccourcis globaux, des images en WebP, une icône dans la zone de notification. Aucune fenêtre, rien à installer.

## Fonctionnalités

- Un raccourci global capture l'écran sous le curseur (`PrintScreen` par défaut).
- Un raccourci global permet de sélectionner une zone en faisant glisser la souris, ou de cliquer sur une fenêtre pour la capturer entière, sur n'importe quel écran (`Shift+PrintScreen` par défaut). Échap ou un clic droit annule. Les deux raccourcis se configurent, voir plus bas.
- Un clic gauche sur l'icône lance directement la sélection de zone, sans raccourci.
- Chaque capture est copiée dans le presse-papier et enregistrée en `.webp` dans un sous-dossier `AAAA-MM`, nommée d'après la fenêtre sous le curseur, la date et la qualité (`Discord_06-14-2026_q90.webp`). Un fichier existant n'est jamais écrasé : ` (1)`, ` (2)`… est ajouté à la place.
- Un aperçu s'affiche brièvement en bas de l'écran. Un clic gauche ouvre l'image, un clic droit ouvre son dossier. Il n'apparaît jamais dans les captures suivantes.
- Menu de l'icône (clic droit) : activer ou désactiver les raccourcis, ouvrir le dossier des captures, Settings, Reload, Website (ouvre nyaruhodo.com), quitter.

## Réglages (`state.json`)

Créé à côté de l'exécutable au premier lancement. Menu de l'icône → Settings l'ouvre (dans le Bloc-notes si aucune application n'est associée aux `.json`) ; Reload applique les modifications. Si le fichier contient du JSON invalide, CASPER Shot utilise les valeurs par défaut et laisse le fichier intact pour que vous puissiez le corriger.

```json
{
  "quality": 90,
  "hotkey_fullscreen": "PrintScreen",
  "hotkey_region": "Shift+PrintScreen",
  "save_path": "%USERPROFILE%\\Pictures\\Screenshots",
  "preview_percent": 40,
  "preview_duration_ms": 2500
}
```

- `quality` : de 0 à 99, WebP avec perte ; 100, sans perte.
- `hotkey_*` : toute combinaison de `Ctrl`/`Alt`/`Shift`/`Win` suivie d'une lettre, d'un chiffre, de F1-F12 ou de `PrintScreen`.
- `save_path` : les variables `%VARIABLE%` sont remplacées ; un chemin relatif part du dossier de l'exécutable. `%USERPROFILE%\Pictures` (ainsi que Desktop, Documents, Downloads, Music et Videos) suit le dossier là où Windows l'a déplacé, OneDrive compris.
- `preview_percent` : taille de l'aperçu, en % de la largeur et de la hauteur de l'écran ; `0` le désactive complètement.
- `preview_duration_ms` : durée d'affichage de l'aperçu avant sa fermeture.

## Les raccourcis ne marchent pas ?

`RegisterHotKey` ne permet qu'à un seul processus de posséder une combinaison de touches donnée dans tout le système. Si une autre application en cours d'exécution utilise déjà la même combinaison (ShareX, Greenshot, le raccourci de capture d'écran de Windows dans Paramètres → Accessibilité → Clavier, etc.), l'enregistrement de CASPER Shot échoue sans message et l'info-bulle de l'icône affiche « hotkeys unavailable » au lieu des raccourcis. Fermez ou reconfigurez l'application qui les occupe, ou choisissez d'autres raccourcis dans `state.json` puis Reload.

## Mise à jour

Lancez `casper_shot_updater.bat` depuis le dossier `CASPER Shot` : il ferme CASPER Shot, télécharge la dernière version, l'extrait sur place et le relance. `state.json` et vos captures ne sont pas touchés.

## Licence

Gratuit pour un usage personnel et non commercial. Copyright (C) 2026 Nyaldee, tous droits réservés — voir [LICENSE](../LICENSE) pour les conditions complètes. À télécharger uniquement depuis les [releases](https://github.com/Nyaldee/nyaruhodo/releases) de ce dépôt.

[Claude](https://claude.com), l'IA d'Anthropic, a contribué au code de ce projet.
