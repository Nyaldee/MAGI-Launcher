# MAGI Launcher

<p align="center">
  <img src="MAGI Launcher.png" alt="MAGI Launcher screenshot">
</p>

*[Read in English](README.md)*

Un lanceur d'applications Windows rapide, tentaculaire et 100% clavier : un raccourci global, taper quelques lettres, Entrée. Il lance des applications, dossiers et raccourcis ; fait aussi calculatrice et prévisualisation de couleurs hexadécimales ; bascule entre les fenêtres ouvertes ; pose des minuteurs rapides ; garde des notes autocollantes ; relance automatiquement ce que vous voulez garder actif ; permet de parcourir et vider la Corbeille ; retrouve un emoji par son nom ; contrôle la lecture média ; et embarque plus de 100 thèmes de couleur. Un petit `.exe` léger, sans installeur et sans rien à configurer en dehors de votre propre liste de raccourcis.

## Fonctionnalités

- Raccourci global pour afficher/masquer le lanceur depuis n'importe où (par défaut `Ctrl+Espace`, configurable)
- Recherche floue sur ta liste d'applis, dossiers et raccourcis
- Calculatrice intégrée (`2*(3+4)` → `= 14`, copie le résultat sur Entrée)
- Aperçu de couleur hexadécimale (`#3a8ea0` remplit la liste de cette couleur, copie le hex sur Entrée)
- Window Switcher — recherche floue et bascule vers n'importe quelle fenêtre ouverte, ou ferme/tue-la sur place
- Timer avec un analyseur de durée (`5m`, `90s`, `1h`) et un easter egg écran de veille DVD-bounce quand il sonne
- Sticky Notes — une liste illimitée et cherchable de notes texte rapides, copie ou supprime sur place
- Auto-restart — choisis une cible à garder en vie ; si son process disparaît (crash ou fermeture manuelle), il est relancé automatiquement en quelques secondes
- Nombre d'objets/poids de la Corbeille affiché en direct, consulte ce qu'elle contient directement dans le lanceur — copie le nom d'un objet ou supprime-le individuellement — vide-la en une touche
- Sélecteur d'emoji — recherche floue sur les noms officiels Unicode, `Entrée` copie l'emoji lui-même
- Contrôle des touches média (lecture/pause, piste suivante/précédente, volume) sans clavier média physique
- Plus de 100 thèmes de couleur intégrés, changeables en direct depuis le lanceur avec aperçu instantané, sans redémarrage
- Tourne en instance unique avec une icône dans la zone de notification (activer/désactiver le hotkey / activer/désactiver l'auto-restart / GitHub / quitter)
- Se rabat sur un comportement « Exécuter » (comme Win+R) pour tout ce qui ne correspond à aucune appli

## Raccourcis clavier

| Touche | Action |
|---|---|
| Raccourci global (par défaut `Ctrl+Espace`) | Afficher / masquer le lanceur |
| Taper | Filtre la liste en direct (recherche floue) |
| `↓` / `↑` ou `Ctrl+S` / `Ctrl+W` | Déplace la sélection bas / haut |
| `→` / `←` ou `Ctrl+D` / `Ctrl+A` | Saute d'une page (10 éléments) en avant / arrière |
| `Entrée` | Lance l'entrée sélectionnée — *(Corbeille, entrée principale)* consulte son contenu au lieu de la vider — *(Corbeille, vue de consultation)* copie le nom complet (avec extension) de l'objet en surbrillance dans le presse-papiers et ferme le lanceur |
| `Shift+Entrée` | Révèle l'entrée sélectionnée dans l'Explorateur au lieu de la lancer — *(Corbeille, entrée principale)* la vide — *(Sticky Notes)* Ouvre `notes.json` dans son éditeur par défaut au lieu de copier |
| `Tab` | Va au Window Switcher — pareil depuis n'importe où dans le lanceur, pas seulement la liste principale |
| `Échap` | Revient à la liste principale depuis n'importe quel mode — ou ferme le lanceur si on y est déjà |
| `Suppr` | *(Window Switcher)* Ferme gentiment la fenêtre en surbrillance (`WM_CLOSE`, comme cliquer sa croix) — *(Sticky Notes)* Supprime la note en surbrillance — *(Auto-restart)* Arrête de surveiller la cible en surbrillance — *(Corbeille, entrée principale)* Vide toute la Corbeille — *(Corbeille, vue de consultation)* Supprime définitivement seulement l'objet en surbrillance — *(Timer, armé ou depuis la saisie de durée)* Annule le compte à rebours |
| `Shift+Suppr` | *(Window Switcher)* Tue de force le process de la fenêtre en surbrillance, pour une qui ne répond plus à `Suppr` — *(Sticky Notes)* Supprime toutes les notes — *(Auto-restart)* Arrête de surveiller toutes les cibles — *(Corbeille, vue de consultation)* Vide toute la Corbeille |
| `Retour arrière` sur une recherche vide | Sort du Window Switcher / du sélecteur de thème / du Timer / des Sticky Notes / d'Auto-restart / de la vue de consultation de la Corbeille / du sélecteur d'emoji |
| Clic gauche sur l'icône tray | Afficher / masquer le lanceur |
| Clic droit sur l'icône tray | Menu Activer/désactiver le hotkey / Activer/désactiver l'auto-restart / GitHub / Quitter |

La barre de recherche garde toujours le focus clavier — chaque touche ci-dessus y est interceptée directement, tu n'as donc jamais besoin de cliquer ou de tabuler ailleurs pour continuer à taper. La souris n'a aucun effet nulle part dans la fenêtre du lanceur (clics, survol, changement de curseur) — seule l'icône du tray y réagit.

## Modes de recherche

Taper quelque chose dans la barre de recherche est interprété, dans l'ordre :

1. **Couleur hexadécimale** (`#fff` ou `#3a8ea0`) — remplit toute la liste de cette couleur en aperçu direct ; `Entrée` copie le code hex dans le presse-papiers.
2. **Expression mathématique** (doit contenir un opérateur, ex: `2+2`, `100/3`) — affiche `= <résultat>` ; `Entrée` copie le résultat. Évaluée via un petit parseur maison à descente récursive, restreint aux nombres et opérateurs arithmétiques — rien qui puisse chercher un nom, appeler quoi que ce soit, ou sortir de l'expression elle-même ne peut jamais s'exécuter.
3. **Nom d'appli** — matching flou : ta recherche doit juste apparaître comme sous-séquence du nom (dans l'ordre, pas forcément consécutive), donc `vsc` trouve « Visual Studio Code ». Résultats classés : correspondance de préfixe exacte en premier, puis sous-chaîne simple, puis correspondances floues (les plus « serrées » classées avant les plus étalées). En cas d'égalité dans le même palier, l'ordre garde celui des entrées dans `apps.json` — donc parmi des correspondances aussi bonnes les unes que les autres, celle qui est plus haut dans `apps.json` apparaît en premier dans les résultats. Il n'y a aucun historique d'usage/frécence derrière tout ça : MAGI ne se souvient jamais de ce que tu as lancé avant, la position dans `apps.json` est le seul départage, toujours le même peu importe la fréquence (ou la récence) avec laquelle tu as choisi quelque chose.
4. **N'importe quoi d'autre** — si rien ne correspond, `Entrée` exécute le texte brut comme la boîte « Exécuter » de Windows (`Win+R`), via la même résolution `ShellExecute`/PATH. Une recherche qui ressemble à un email (`quelqu'un@exemple.com`) est lancée comme `mailto:` au lieu d'échouer comme un chemin de fichier bidon.

## Window Switcher

Appuie sur `Tab` depuis n'importe où dans le lanceur pour lister toutes les fenêtres top-level ouvertes (titre, filtré de la même façon floue que les applis) :

- `Entrée` active la fenêtre en surbrillance
- `Suppr` la ferme gentiment (`WM_CLOSE`, comme cliquer la croix) et reste dans le switcher pour la suivante
- `Shift+Suppr` tue de force son process (`TerminateProcess`) pour une fenêtre qui ne répond plus à `Suppr`
- `Échap` sort du switcher vers la liste principale sans toucher à aucune fenêtre

`Shift+Suppr` est volontairement à un modificateur de distance de la simple fermeture par `Suppr` : beaucoup de fenêtres — toutes les fenêtres de dossier de l'Explorateur de fichiers, par exemple — tournent dans le même process que le bureau et la barre des tâches (sauf si « Lancer les fenêtres de dossiers dans un processus distinct » est activé), donc un `TerminateProcess` sur l'une d'elles emporte tout `explorer.exe`, pas juste la fenêtre en surbrillance. N'y recours que quand `Suppr` ne passe vraiment pas.

## Timer

Ajoute une entrée `"magi:timer"` (voir plus bas) pour le débloquer — le placeholder de la barre de recherche passe à `Type a duration (5m, 90s, 1h...)` tant que tu y es. Tape une durée (`5m`, `90s`, `1h`, ou un nombre seul qui vaut minutes par défaut) et appuie sur `Entrée` pour armer un compte à rebours, affiché en direct à côté de « Timer » dans la liste principale (`Timer: --:--` tant qu'il est inactif). Quand il arrive à zéro, le popup se met à rebondir sur l'écran façon écran de veille DVD, changeant de thème à chaque rebond sur un mur — dismiss-le avec le raccourci global, un clic souris n'importe où dessus, ou n'importe quelle touche (`Échap`, `Tab`...) tant qu'il a le focus clavier. Changé d'avis avant qu'il n'alerte ? Appuie sur `Suppr` pour annuler le compte à rebours, que tu sois encore dans la saisie de durée ou que « Timer » soit en surbrillance dans la recherche principale.

## Sticky Notes

Ajoute une entrée `"magi:notes"` (voir plus bas) pour la débloquer — un bloc-notes illimité de notes texte rapides, stocké dans `notes.json` à côté de `apps.json`/`themes.json`. La sélectionner liste toutes les notes (les plus récentes en premier, affichées en direct à côté de « Notes » dans la liste principale), cherchable de façon floue exactement comme le Window Switcher :

- Tape un texte qui correspond à une note existante, `Entrée` la copie dans le presse-papiers et ferme le lanceur
- Tape un texte qui ne correspond à rien, `Entrée` l'ajoute comme nouvelle note et reste ouvert pour en enchaîner d'autres
- `Suppr` retire la note en surbrillance, `Shift+Suppr` les efface toutes
- `Shift+Entrée` ouvre `notes.json` directement dans son éditeur associé au lieu de copier — pratique pour éditer une note à la main (multi-lignes, réordonner...)
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide ferme le picker

## Auto-restart

Ajoute une entrée `"magi:auto-restart"` (voir plus bas) pour la débloquer — une liste de cibles (n'importe quel `path`, même format qu'une entrée `apps.json`, arguments compris — rien n'est rejeté a priori) que MAGI garde en vie en tâche de fond, stockée dans `restart.json` à côté de `apps.json`/`themes.json`. Un thread dédié vérifie toutes les quelques secondes si le process de chaque cible surveillée tourne encore (par nom d'exécutable, pas en gardant un handle) et la relance dès que ce n'est plus le cas — crash, ou toi qui la fermes volontairement, ça ne fait aucune différence, elle revient dans les deux cas. Sélectionner l'entrée liste toutes les cibles actuellement surveillées (affichées en direct à côté de « Auto-restart » dans la liste principale sous la forme `Auto-restart: N`, `0` si vide), cherchable de façon floue exactement comme les Sticky Notes, chacune préfixée de `★` (en cours d'exécution) ou `☆` (pas en cours d'exécution) :

- Tape un texte qui correspond à une cible existante, `Entrée` ne fait rien (il n'y a rien d'utile à faire sur une entrée existante ici à part la retirer, voir `Suppr` ci-dessous)
- Tape un chemin qui ne correspond à rien, `Entrée` l'ajoute à la liste de surveillance et reste ouvert pour en enchaîner d'autres — tape-le tel quel, comme tu le collerais depuis la barre d'adresse de l'Explorateur (antislash simples) : ce champ est du texte brut, pas du JSON, aucun doublement n'est nécessaire ici. Les antislash doublés que tu verrais en ouvrant ensuite le vrai `restart.json` ne sont que son échappement JSON normal à l'écriture (la même règle que suit `path` dans `apps.json`, voir plus bas) — MAGI les relit correctement dans les deux cas.
- `Suppr` arrête de surveiller la cible en surbrillance (ne ferme **pas** ni ne tue l'appli elle-même, juste fin de la surveillance)
- `Shift+Suppr` arrête de surveiller toutes les cibles d'un coup (vide toute la liste, comme pour Sticky Notes)
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide ferme le picker

Une cible n'a pas besoin d'exister par ailleurs dans `apps.json` — les deux listes sont entièrement indépendantes, donc la même appli peut être à la fois une entrée normale lançable, une cible auto-restart surveillée, les deux, ou ni l'une ni l'autre. Comme la détection est purement « ce nom d'exécutable tourne-t-il, tout court », MAGI ne peut pas distinguer un vrai crash d'une fermeture volontaire de ta part — si c'est dans la liste, ça revient, point final. Il n'y a pas non plus de tentative de détecter une cible gelée-mais-toujours-active (« ne répond pas ») pour la tuer de force : ça risquerait de tuer quelque chose qui était juste momentanément occupé et sur le point de se rétablir tout seul, ce qui serait pire que de ne rien faire. Le menu du tray a aussi son propre bascule « Activer/désactiver Auto-restart », pour mettre en pause tout le superviseur sans toucher à la liste de surveillance elle-même (même principe que désactiver le hotkey).

## Corbeille

L'entrée intégrée `"magi:empty-recycle-bin"` (voir plus bas) affiche le nombre d'objets/poids en direct à côté de son nom, actualisé immédiatement dès que tu la vides — rouvrir le lanceur juste après ne montre jamais de restes obsolètes. Appuyer sur `Entrée` dessus ouvre une liste cherchable de façon floue de ce qu'il y a vraiment dans la Corbeille en ce moment (tous les lecteurs confondus, lu directement depuis `$Recycle.Bin` — aucune fenêtre Explorateur impliquée) :

- Tape pour filtrer la liste des objets supprimés de façon floue, comme partout ailleurs
- `Entrée` sur un objet en surbrillance copie son nom complet (avec extension) dans le presse-papiers et ferme le lanceur
- `Suppr` supprime définitivement seulement l'objet en surbrillance de la Corbeille — le reste n'est pas touché
- `Shift+Suppr` vide toute la Corbeille, depuis cette vue aussi
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

Pour vider la Corbeille elle-même sans l'ouvrir, utilise `Shift+Entrée` ou `Suppr` directement sur l'entrée de la liste principale — volontairement une touche différente de celle qui ouvre la vue de consultation, pour qu'un simple coup d'œil ne puisse jamais la vider par accident.

## Emoji

Ajoute une entrée `"magi:emoji"` (voir plus bas) pour le débloquer — cherche de façon floue dans les noms officiels Unicode des emoji ("fire", "red heart", "grinning face"...) et appuie sur `Entrée` pour copier l'emoji lui-même dans le presse-papiers et fermer le lanceur. Aucune liste embarquée, aucun JSON : ça lit `emoji-test.txt`, le fichier texte de référence qu'Unicode publie lui-même sur [unicode.org/Public/emoji/latest/emoji-test.txt](https://www.unicode.org/Public/emoji/latest/emoji-test.txt), placé à côté de l'exécutable comme `apps.json`/`themes.json`. La liste principale affiche `Emoji: Version 17.0` (ou la version que déclare le fichier) en direct à côté de son nom ; si le fichier est absent, elle affiche `Emoji: missing emoji-test.txt` à la place et `Entrée` dessus ne fait rien — télécharge une copie depuis le lien ci-dessus et dépose-la à côté de l'`.exe` (ou fais **Reload** si le lanceur tourne déjà) pour le débloquer.

- Tape pour filtrer par nom de façon floue, comme partout ailleurs
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

Pour passer à un set d'emoji plus récent plus tard, remplace juste `emoji-test.txt` par une copie plus fraîche depuis Unicode et fais **Reload** — aucune recompilation nécessaire. Les emoji tout juste ajoutés peuvent encore s'afficher comme un carré vide dans la liste tant que la police emoji de Windows n'a pas suivi (copier fonctionne quand même).

## Configuration

`apps.json` et `themes.json` vivent à côté de l'exécutable — jamais embarqués dedans — donc tu peux les éditer à la main sans reconstruire. Utilise **Reload** (une entrée `"magi:reload"`) pour prendre en compte les changements sans redémarrer l'appli.

`notes.json`/`restart.json` vivent là aussi, mais c'est un genre de fichier différent : `notes.json` est un simple tableau JSON de chaînes (`["note 1", "note 2"]`), `restart.json` un simple tableau JSON de chaînes `path` (`["A:\\Apps\\Foo\\Foo.exe"]`) — les deux créés et réécrits automatiquement par l'appli elle-même à chaque ajout/suppression d'entrée dans Sticky Notes/Auto-restart, donc la copie en mémoire du lanceur fait normalement toujours foi. Pas faits pour être édités à la main, mais **Reload** relit aussi les deux depuis le disque (en plus de `apps.json`/`themes.json`), donc une modification manuelle de l'un ou l'autre est quand même prise en compte sans redémarrer.

`emoji-test.txt` est encore un autre genre : un fichier texte de référence tel quel venu d'Unicode (voir [Emoji](#emoji) plus haut), ni du JSON ni écrit par MAGI lui-même — tu remplaces le fichier entier pour le mettre à jour, rien à éditer entrée par entrée dedans. Optionnel : le sélecteur d'emoji reste juste verrouillé (avec un `Emoji: missing emoji-test.txt` explicite dans la liste principale) s'il est absent.

### `apps.json`

```json
{
  "hotkey": "ctrl+space",
  "apps": [
    { "name": "Notepad", "path": "%windir%\\system32\\notepad.exe" },
    { "name": "Command Prompt", "path": "%windir%\\system32\\cmd.exe", "cwd": "%HOMEDRIVE%%HOMEPATH%" },
    { "name": "Some background script", "path": "A:\\Scripts\\thing.ps1", "hidden": true }
  ]
}
```

- **`hotkey`** — une spec du genre `"ctrl+space"`, `"ctrl+alt+f"`, `"win+e"`, `"f14"`. Supporte `ctrl`/`control`, `alt`, `shift`, `win`/`super`, `space`, `enter`/`return`, `tab`, `esc`/`escape`, `f1`–`f24`, et les caractères seuls.
- **`name`** (obligatoire) — nom affiché, cherchable de façon floue.
- **`path`** (obligatoire) — un chemin simple, un chemin avec arguments (`"app.exe --flag"`), une URI shell (`ms-settings:...`, `shell:RecycleBinFolder`...), ou une [entrée spéciale `magi:`](#entrées-spéciales) ci-dessous. Tout passe par `ShellExecute`, donc tout ce qu'Explorer sait ouvrir (y compris les types de documents résolus par association de fichier, comme les fichiers `.msc`) fonctionne. Les antislashs doivent être doublés (`"A:\\Apps\\Foo.exe"`) puisque `\` est un caractère d'échappement JSON — un seul `\` est du JSON invalide et peut corrompre le chemin en silence (`\t`, `\n`... sont de vraies séquences d'échappement). Les slashs (`"A:/Apps/Foo.exe"`) marchent aussi et n'ont besoin d'aucun échappement — Windows accepte les deux.
- **`cwd`** (optionnel) — dossier de travail. Par défaut le dossier de la cible elle-même (comme un double-clic Explorer), sauf pour les entrées type `cmd.exe`/`powershell.exe` où tu voudras généralement le préciser explicitement (sinon elles démarrent dans `system32`).
- **`hidden`** (optionnel, par défaut `false`) — lance sans fenêtre visible (`SW_HIDE`), pour les scripts qui n'ont rien à afficher.

#### Entrées spéciales

| `path` | Effet |
|---|---|
| `magi:reload` | Recharge `apps.json`/`themes.json`/`notes.json`/`restart.json`/`emoji-test.txt` sur place, sans redémarrage |
| `magi:theme-picker` | Entre dans un sélecteur de thème en direct (voir plus bas) |
| `magi:timer` | Entre dans la saisie de durée du timer ; affiche `<nom>: --:--` tant qu'il est inactif, le compte à rebours en direct une fois armé |
| `magi:notes` | Entre dans le picker Sticky Notes (voir plus haut) ; affiche `<nom>:` si vide, `<nom>: <dernière note>` sinon |
| `magi:auto-restart` | Entre dans le picker Auto-restart (voir plus haut) ; affiche `<nom>: N` pour le nombre de cibles surveillées, `0` si vide |
| `magi:open-folder` | Ouvre le dossier contenant MAGI Launcher dans l'Explorateur — résolu à chaque lancement, suit automatiquement si le dossier est déplacé |
| `magi:empty-recycle-bin` | Affiche `<nom>: N objets, X Mo` si non vide ; `Entrée` consulte son contenu (voir [Corbeille](#corbeille) plus haut), `Shift+Entrée` ou `Suppr` la vide |
| `magi:emoji` | Entre dans le sélecteur d'emoji (voir plus haut) ; affiche `<nom>: Version X.Y` depuis `emoji-test.txt`, ou `<nom>: missing emoji-test.txt` (Entrée ne fait rien) si le fichier est absent |
| `magi:media-play-pause`, `magi:media-next`, `magi:media-previous`, `magi:media-stop`, `magi:media-volume-mute`, `magi:media-volume-down`, `magi:media-volume-up` | Envoie la touche média virtuelle correspondante, routée comme le ferait une vraie touche physique (session média globale, pas juste cette fenêtre) |

Un vrai dialogue « Arrêter Windows » (le même que tu obtiens avec `Alt+F4` sur le bureau) peut aussi être ajouté comme une entrée normale, sans code spécial nécessaire :

```json
{
  "name": "Shutdown",
  "path": "%windir%\\system32\\WindowsPowerShell\\v1.0\\powershell.exe -command \"(New-Object -ComObject Shell.Application).ShutdownWindows()\"",
  "hidden": true
}
```

### `themes.json`

```json
{
  "theme": "arc-dark",
  "font_family": "Segoe UI",
  "placeholder_text": "Type to search...",
  "show_clock": true,
  "window_width_fraction": 0.30,
  "border": 3,
  "themes": {
    "arc-dark": {
      "search_background": "#404552",
      "search_text": "#7c818c",
      "list_background": "#383c4a",
      "list_text": "#d3dae3",
      "selected_background": "#5294e2",
      "selected_text": "#ffffff",
      "border": "#4b5162"
    }
  }
}
```

Les clés à la racine s'appliquent quel que soit le thème actif :

- **`theme`** — nom de l'entrée active dans `themes`
- **`font_family`** — omets/vide pour garder la police par défaut de l'OS. Police recommandée : [SGr-Iosevka-Regular.ttc](https://github.com/be5invis/iosevka)
- **`placeholder_text`** — affiché dans la barre de recherche quand elle est vide
- **`show_clock`** — affiche l'heure actuelle (au format court de Windows de l'utilisateur) à côté de la barre de recherche
- **`window_width_fraction`** — fraction de la largeur de l'écran occupée par le popup (hauteur/tailles de police suivent, en gardant un ratio 16:9)
- **`border`** — épaisseur de bordure simulée en pixels

Livré avec 100+ thèmes intégrés (surtout des palettes de personnages/jeux) dans le dict `themes` — ouvre le lanceur et sélectionne l'entrée `Themes` (`magi:theme-picker`) pour les prévisualiser et en changer en direct, sans redémarrage nécessaire. Le picker s'ouvre sur le thème actuellement actif, pas le premier par ordre alphabétique. En sélectionner un le réécrit dans `themes.json`.

## Crédits

Construit avec [Claude](https://claude.com) (l'assistant de code IA d'Anthropic).

## Licence

Copyright (C) 2026 Nyaldee. Distribué sous licence [GNU General Public License v3.0](LICENSE) — voir le fichier `LICENSE` pour le texte complet.
