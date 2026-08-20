# MAGI Launcher

<p align="center">
  <img src="MAGI Launcher.png" alt="MAGI Launcher screenshot">
</p>

*[Read in English](README.md)*

Un lanceur d'applications Windows rapide, tentaculaire et 100% clavier : un raccourci global, taper quelques lettres, Entrée. Il lance des applications, dossiers et raccourcis ; fait aussi calculatrice et prévisualisation de couleurs hexadécimales ; bascule entre les fenêtres ouvertes ; pose des minuteurs rapides ; garde des notes autocollantes ; relance automatiquement ce que vous voulez garder actif ; permet de parcourir et vider la Corbeille ; retrouve un emoji par son nom ; contrôle la lecture média ; garde un historique de presse-papier optionnel en RAM ; éjecte des clés USB même quand Windows lui-même refuse parfois de le faire ; et embarque plus de 100 thèmes de couleur. Un petit `.exe` léger, sans installeur et sans rien à configurer en dehors de votre propre liste de raccourcis.

## Fonctionnalités

- Raccourci global pour afficher/masquer le lanceur depuis n'importe où (par défaut `Ctrl+Espace`, configurable)
- Recherche floue sur ta liste d'applis, dossiers et raccourcis
- Un dossier `shortcuts/` à côté de l'exécutable — dépose n'importe quel fichier dedans (`.lnk`, `.bat`, `.cmd`, `.vbs`, n'importe quoi) et il devient automatiquement une entrée lançable, listée après celles d'`apps.json`
- Calculatrice intégrée (`2*(3+4)` → `= 14`, copie le résultat sur Entrée)
- Aperçu de couleur hexadécimale (`#3a8ea0` remplit la liste de cette couleur, copie le hex sur Entrée)
- Window Switcher — recherche floue et bascule vers n'importe quelle fenêtre ouverte, ou ferme/tue-la sur place
- Timer avec un analyseur de durée (`5m`, `90s`, `1h`) et un easter egg écran de veille DVD-bounce quand il sonne
- Sticky Notes — une liste illimitée et cherchable de notes texte rapides, copie ou supprime sur place
- Auto-restart — choisis une cible à garder en vie ; si son process disparaît (crash ou fermeture manuelle), il est relancé automatiquement en quelques secondes
- Nombre d'objets/poids de la Corbeille affiché en direct, consulte ce qu'elle contient directement dans le lanceur — copie le nom d'un objet ou supprime-le individuellement — vide-la en une touche
- Sélecteur d'emoji — recherche floue sur les noms officiels Unicode, `Entrée` copie l'emoji lui-même
- Copy History — historique du presse-papier optionnel, uniquement en RAM (texte seulement) ; parcours et re-copie depuis le lanceur, jamais écrit sur disque
- Eject — liste cherchable des clés/disques USB branchés, `Entrée` éjecte celui en surbrillance sur place (`Shift+Suppr` pour forcer si quelque chose l'utilise) ; marche même quand l'option « Éjecter » de Windows est absente ou grisée
- Contrôle des touches média (lecture/pause, piste suivante/précédente, volume) sans clavier média physique
- Plus de 100 thèmes de couleur intégrés, changeables en direct depuis le lanceur avec aperçu instantané, sans redémarrage
- Redimensionne le popup et sa bordure en direct au clavier (`Ctrl+1`–`Ctrl+0` / `Ctrl+-`/`Ctrl+=`), persisté aussitôt
- Tourne en instance unique avec une icône dans la zone de notification (activer/désactiver le hotkey / activer/désactiver l'auto-restart / activer/désactiver Copy History / GitHub / quitter)
- Se rabat sur un comportement « Exécuter » (comme Win+R) pour tout ce qui ne correspond à aucune appli

## Raccourcis clavier

| Touche | Action |
|---|---|
| Raccourci global (par défaut `Ctrl+Espace`) | Afficher / masquer le lanceur |
| Taper | Filtre la liste en direct (recherche floue) |
| `↑` / `↓` ou `Ctrl+W` / `Ctrl+S` | Déplace la sélection haut / bas |
| `←` / `→` ou `Ctrl+A` / `Ctrl+D` | Saute d'une page (10 éléments) en arrière / avant |
| `Entrée` | Lance l'entrée sélectionnée — *(Corbeille, entrée principale)* consulte son contenu au lieu de la vider — *(Corbeille, vue de consultation)* copie le nom complet (avec extension) de l'objet en surbrillance dans le presse-papiers et ferme le lanceur — *(Copy History)* re-copie l'entrée en surbrillance et ferme le lanceur, sans la rajouter à l'historique — *(Eject)* éjecte le disque en surbrillance UNIQUEMENT si rien ne l'utilise, sinon ne fait rien (voir `Shift+Suppr` pour forcer) ; le lanceur reste ouvert dans les deux cas pour en éjecter plusieurs à la suite |
| `Shift+Entrée` | Révèle l'entrée sélectionnée dans l'Explorateur au lieu de la lancer — *(Corbeille, entrée principale)* la vide (le lanceur reste ouvert) — *(Sticky Notes)* Ouvre `notes.json` dans son éditeur par défaut au lieu de copier |
| `Tab` | Va au Window Switcher — pareil depuis n'importe où dans le lanceur, pas seulement la liste principale |
| `Échap` | Revient à la liste principale depuis n'importe quel mode — ou ferme le lanceur si on y est déjà. Pendant le rebond DVD du Timer, arrête le rebond et revient à la liste principale au lieu de fermer |
| `Suppr` | *(Window Switcher)* Ferme gentiment la fenêtre en surbrillance (`WM_CLOSE`, comme cliquer sa croix) — *(Sticky Notes, depuis le picker)* Supprime la note en surbrillance — *(liste principale, sur l'entrée « Note »)* Supprime directement la note la plus récente, sans ouvrir le picker — *(Auto-restart)* Arrête de surveiller la cible en surbrillance — *(Copy History)* Supprime l'entrée en surbrillance — *(Corbeille, entrée principale)* Vide toute la Corbeille (le lanceur reste ouvert) — *(Corbeille, vue de consultation)* Supprime définitivement seulement l'objet en surbrillance — *(Timer, armé ou depuis la saisie de durée)* Annule le compte à rebours |
| `Shift+Suppr` | *(Window Switcher)* Tue de force le process de la fenêtre en surbrillance, pour une qui ne répond plus à `Suppr` — *(Sticky Notes)* Supprime toutes les notes — *(Auto-restart)* Arrête de surveiller toutes les cibles — *(Copy History)* Vide tout l'historique — *(Corbeille, vue de consultation)* Vide toute la Corbeille (le lanceur reste ouvert) — *(Eject)* Force l'éjection du disque en surbrillance même si quelque chose l'utilise encore ; `Suppr` seul ne fait rien ici |
| `Ctrl+1`–`Ctrl+9` / `Ctrl+0` | Redimensionne le popup en direct à 10%–90% / 100% de la largeur de l'écran, persisté aussitôt dans `themes.json` |
| `Ctrl+-` / `Ctrl+=` | Réduit / agrandit la bordure de 1px, en direct, persisté aussitôt |
| `Retour arrière` sur une recherche vide | Sort du Window Switcher / du sélecteur de thème / du Timer / des Sticky Notes / d'Auto-restart / de la vue de consultation de la Corbeille / du sélecteur d'emoji / de Copy History / d'Eject |
| Clic gauche sur l'icône tray | Afficher / masquer le lanceur |
| Clic droit sur l'icône tray | Menu Activer/désactiver le hotkey / Activer/désactiver l'auto-restart / Activer/désactiver Copy History / GitHub / Quitter |

La barre de recherche garde toujours le focus clavier — chaque touche ci-dessus y est interceptée directement, tu n'as donc jamais besoin de cliquer ou de tabuler ailleurs pour continuer à taper. La souris n'a aucun effet nulle part dans la fenêtre du lanceur (clics, survol, changement de curseur) — seule l'icône du tray y réagit. `Alt+F4` ne ferme jamais le process du lanceur — ça masque juste le popup, comme `Échap` ; le lanceur ne quitte réellement que via « Quitter » dans le menu du tray.

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

Ajoute une entrée `"magi:notes"` (voir plus bas) pour la débloquer — un bloc-notes illimité de notes texte rapides, stocké dans `notes.json` à côté de `apps.json`/`themes.json`. La sélectionner liste toutes les notes (les plus récentes en premier, affichées en direct à côté de « Notes » dans la liste principale), cherchable de façon floue exactement comme le Window Switcher. Le placeholder de la barre de recherche passe à `Type a note...` tant que tu y es.

- Tape un texte qui correspond à une note existante, `Entrée` la copie dans le presse-papiers et ferme le lanceur
- Tape un texte qui ne correspond à rien, `Entrée` l'ajoute comme nouvelle note et reste ouvert pour en enchaîner d'autres
- `Suppr` retire la note en surbrillance, `Shift+Suppr` les efface toutes
- `Shift+Entrée` ouvre `notes.json` directement dans son éditeur associé au lieu de copier — pratique pour éditer une note à la main (multi-lignes, réordonner...)
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide ferme le picker

Pas besoin d'ouvrir le picker pour retirer la note la plus récente : `Suppr` directement sur l'entrée « Note » de la liste principale, comme annuler le Timer sans ouvrir sa saisie.

## Auto-restart

Ajoute une entrée `"magi:auto-restart"` (voir plus bas) pour la débloquer — une liste de cibles (n'importe quel `path`, même format qu'une entrée `apps.json`, arguments compris — rien n'est rejeté a priori) que MAGI garde en vie en tâche de fond, stockée dans `restart.json` à côté de `apps.json`/`themes.json`. Un thread dédié vérifie toutes les quelques secondes si le process de chaque cible surveillée tourne encore (par nom d'exécutable, pas en gardant un handle) et la relance dès que ce n'est plus le cas — crash, ou toi qui la fermes volontairement, ça ne fait aucune différence, elle revient dans les deux cas. Sélectionner l'entrée liste toutes les cibles actuellement surveillées (affichées en direct à côté de « Auto-restart » dans la liste principale sous la forme `Auto-restart: N`, `0` si vide), cherchable de façon floue exactement comme les Sticky Notes, chacune préfixée de `★` (en cours d'exécution) ou `☆` (pas en cours d'exécution). Le placeholder de la barre de recherche passe à `Type a target to watch...` tant que tu y es :

- Tape un texte qui correspond à une cible existante, `Entrée` ne fait rien (il n'y a rien d'utile à faire sur une entrée existante ici à part la retirer, voir `Suppr` ci-dessous)
- Tape un chemin qui ne correspond à rien, `Entrée` l'ajoute à la liste de surveillance et reste ouvert pour en enchaîner d'autres — tape-le tel quel, comme tu le collerais depuis la barre d'adresse de l'Explorateur (antislash simples) : ce champ est du texte brut, pas du JSON, aucun doublement n'est nécessaire ici. Les antislash doublés que tu verrais en ouvrant ensuite le vrai `restart.json` ne sont que son échappement JSON normal à l'écriture (la même règle que suit `path` dans `apps.json`, voir plus bas) — MAGI les relit correctement dans les deux cas.
- `Suppr` arrête de surveiller la cible en surbrillance (ne ferme **pas** ni ne tue l'appli elle-même, juste fin de la surveillance)
- `Shift+Suppr` arrête de surveiller toutes les cibles d'un coup (vide toute la liste, comme pour Sticky Notes)
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide ferme le picker

Une cible n'a pas besoin d'exister par ailleurs dans `apps.json` — les deux listes sont entièrement indépendantes, donc la même appli peut être à la fois une entrée normale lançable, une cible auto-restart surveillée, les deux, ou ni l'une ni l'autre. Comme la détection est purement « ce nom d'exécutable tourne-t-il, tout court », MAGI ne peut pas distinguer un vrai crash d'une fermeture volontaire de ta part — si c'est dans la liste, ça revient, point final. Il n'y a pas non plus de tentative de détecter une cible gelée-mais-toujours-active (« ne répond pas ») pour la tuer de force : ça risquerait de tuer quelque chose qui était juste momentanément occupé et sur le point de se rétablir tout seul, ce qui serait pire que de ne rien faire. Le menu du tray a aussi son propre bascule « Activer/désactiver Auto-restart », pour mettre en pause tout le superviseur sans toucher à la liste de surveillance elle-même (même principe que désactiver le hotkey).

## Corbeille

L'entrée intégrée `"magi:empty-recycle-bin"` (voir plus bas) affiche le nombre d'objets/poids en direct à côté de son nom, actualisé immédiatement dès que tu la vides — rouvrir le lanceur juste après ne montre jamais de restes obsolètes. Appuyer sur `Entrée` dessus ouvre une liste cherchable de façon floue de ce qu'il y a vraiment dans la Corbeille en ce moment (tous les lecteurs confondus, lu directement depuis `$Recycle.Bin`, hors du thread d'interface pour qu'une Corbeille lente/volumineuse ne gèle jamais le lanceur — aucune fenêtre Explorateur impliquée) :

- Tape pour filtrer la liste des objets supprimés de façon floue, comme partout ailleurs
- `Entrée` sur un objet en surbrillance copie son nom complet (avec extension) dans le presse-papiers et ferme le lanceur
- `Suppr` supprime définitivement seulement l'objet en surbrillance de la Corbeille — le reste n'est pas touché
- `Shift+Suppr` vide toute la Corbeille, depuis cette vue aussi — le lanceur reste ouvert, la liste (désormais vide) se rafraîchit sur place
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

Pour vider la Corbeille elle-même sans l'ouvrir, utilise `Shift+Entrée` ou `Suppr` directement sur l'entrée de la liste principale — volontairement une touche différente de celle qui ouvre la vue de consultation, pour qu'un simple coup d'œil ne puisse jamais la vider par accident. Dans les deux cas, le lanceur reste ouvert ; seul le compte à côté de l'entrée se met à jour.

## Emoji

Ajoute une entrée `"magi:emoji"` (voir plus bas) pour le débloquer — cherche de façon floue dans les noms officiels Unicode des emoji ("fire", "red heart", "grinning face"...) et appuie sur `Entrée` pour copier l'emoji lui-même dans le presse-papiers et fermer le lanceur. Aucune liste embarquée, aucun JSON : ça lit `emoji-test.txt`, le fichier texte de référence qu'Unicode publie lui-même sur [unicode.org/Public/emoji/latest/emoji-test.txt](https://www.unicode.org/Public/emoji/latest/emoji-test.txt), placé à côté de l'exécutable comme `apps.json`/`themes.json`. La liste principale affiche `Emoji: Version 17.0` (ou la version que déclare le fichier) en direct à côté de son nom ; si le fichier est absent, elle affiche `Emoji: missing emoji-test.txt` à la place et `Entrée` dessus ne fait rien — télécharge une copie depuis le lien ci-dessus et dépose-la à côté de l'`.exe` (ou fais **Reload** si le lanceur tourne déjà) pour le débloquer.

- Tape pour filtrer par nom de façon floue, comme partout ailleurs
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

Pour passer à un set d'emoji plus récent plus tard, remplace juste `emoji-test.txt` par une copie plus fraîche depuis Unicode et fais **Reload** — aucune recompilation nécessaire. Les emoji tout juste ajoutés peuvent encore s'afficher comme un carré vide dans la liste tant que la police emoji de Windows n'a pas suivi (copier fonctionne quand même).

## Copy History

Optionnel (désactivé par défaut) — active-le depuis le menu du tray (« Enable/Disable Copy History »), ou ajoute une entrée `"magi:copy-history"` (voir plus bas) pour le parcourir depuis le lanceur lui-même. Tant que c'est activé, tout texte copié n'importe où sur le système est enregistré, le plus récent en premier, affiché en direct à côté de « Copy History » dans la liste principale sous forme d'un compte (ou `disabled` tant que la bascule est désactivée).

- Tape pour filtrer de façon floue les copies passées, comme partout ailleurs
- `Entrée` sur une entrée en surbrillance la recopie dans le presse-papiers et ferme le lanceur — sans ajouter de doublon pour cette re-copie
- `Suppr` retire l'entrée en surbrillance, `Shift+Suppr` vide tout l'historique
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

**Où c'est stocké :** nulle part sur disque, jamais. L'historique vit uniquement dans la mémoire (RAM) du process du lanceur lui-même, plafonné à 1 000 000 caractères au total (les entrées les plus anciennes sont supprimées en premier une fois plein) — éteins le PC et c'est parti sans aucune trace, aucun fichier à trouver, aucune télémétrie, rien d'autre que ce process ne peut le lire. La mémoire de chaque entrée est épinglée avec `VirtualLock` pour que Windows ne la swappe jamais sur le disque sous pression mémoire, et elle est écrasée avec des zéros dès qu'elle est libérée (évincée, supprimée, ou l'appli fermée). Volontairement **pas** chiffré : une clé de déchiffrement vivant dans le même process n'arrêterait rien qui puisse déjà lire la mémoire du process, et un buffer chiffré en RAM ressemble davantage à la façon dont un infostealer cache ses propres données capturées qu'à un gestionnaire de presse-papier ordinaire — une mémoire simple, verrouillée, jamais persistée et effacée à la libération est à la fois plus simple et tout aussi sûre ici.

L'état des trois bascules du tray (`hotkey_enabled`/`auto_restart_enabled`/`copy_history_enabled`) est persisté dans `apps.json` (voir plus bas) dès que tu les changes, pour qu'elles survivent à un redémarrage.

## Eject

Ajoute une entrée `"magi:eject"` (voir plus bas) pour le débloquer — une liste cherchable de façon floue de chaque disque branché sur un bus USB en ce moment.

- Tape pour filtrer la liste de façon floue, comme partout ailleurs
- `Entrée` sur un disque en surbrillance l'éjecte *seulement si rien ne l'utilise en ce moment* — en cas de succès il disparaît de la liste et le lanceur reste ouvert, donc éjecter plusieurs disques à la suite n'oblige jamais à rouvrir le picker. Si quelque chose a encore un handle ouvert dessus (antivirus/indexeur en plein scan, une appli avec un fichier ouvert...), `Entrée` ne fait rien du tout et laisse le disque intact — voir `Shift+Suppr` plus bas pour forcer quand même
- `Shift+Suppr` force l'éjection du disque en surbrillance même si quelque chose l'utilise encore — voir l'avertissement plus bas avant de s'appuyer là-dessus
- `Tab` / `Échap` / `Retour arrière` sur une recherche vide sort vers la liste principale

Ne liste que les disques sur un bus USB (vérifié directement contre le périphérique, pas via le flag amovible/fixe de `GetDriveTypeW`, qui classe beaucoup de boîtiers USB-SATA/UASP externes comme « fixes ») — jamais le disque système, jamais un disque interne secondaire.

Volontairement **pas** le même mécanisme que l'icône « Retirer le périphérique en toute sécurité » de Windows (`CM_Request_Device_Eject`), qui décide si un périphérique est éjectable en se fiant à un flag de capacité posé par son pilote — beaucoup de boîtiers externes ne le posent jamais correctement, donc le menu de Windows finit par ne pas proposer l'option, ou l'affiche grisée, pour un disque qui pourtant fonctionne très bien au quotidien. Ceci verrouille et démonte directement le volume de la lettre de lecteur (`FSCTL_LOCK_VOLUME`/`FSCTL_DISMOUNT_VOLUME` + `IOCTL_STORAGE_EJECT_MEDIA`), le même chemin qu'empruntent la plupart des utilitaires tiers d'éjection USB — ça marche précisément dans les cas où l'option de Windows n'apparaît pas du tout.

**`Shift+Suppr` force l'éjection, ça ne demande pas la permission avant.** Contrairement à `Entrée` seule (qui recule proprement dès que le verrouillage du volume est refusé), le chemin forcé saute complètement cette vérification : l'étape de démontage réussit même si un autre process a encore un fichier ouvert sur le disque, et son handle est simplement invalidé (il prend une erreur d'E/S) plutôt que de bloquer l'éjection. Un scan en arrière-plan (antivirus, indexeur de recherche) interrompu en pleine *lecture* de cette façon est sans danger — rien n'était modifié, ça échoue juste proprement de son côté. Mais un fichier réellement en train d'être *écrit* sur le disque à cet instant précis (une copie en cours, une sauvegarde en cours) finira tronqué/corrompu, sans aucun avertissement au préalable — il n'y a aucune vérification "est-ce que quelque chose écrit en ce moment", forcé veut dire forcé.

## Configuration

`apps.json` et `themes.json` vivent à côté de l'exécutable — jamais embarqués dedans — donc tu peux les éditer à la main sans reconstruire. Utilise **Reload** (une entrée `"magi:reload"`) pour prendre en compte les changements sans redémarrer l'appli.

`notes.json`/`restart.json` vivent là aussi, mais c'est un genre de fichier différent : `notes.json` est un simple tableau JSON de chaînes (`["note 1", "note 2"]`), `restart.json` un simple tableau JSON de chaînes `path` (`["A:\\Apps\\Foo\\Foo.exe"]`) — les deux créés et réécrits automatiquement par l'appli elle-même à chaque ajout/suppression d'entrée dans Sticky Notes/Auto-restart, donc la copie en mémoire du lanceur fait normalement toujours foi. Pas faits pour être édités à la main, mais **Reload** relit aussi les deux depuis le disque (en plus de `apps.json`/`themes.json`), donc une modification manuelle de l'un ou l'autre est quand même prise en compte sans redémarrer.

`emoji-test.txt` est encore un autre genre : un fichier texte de référence tel quel venu d'Unicode (voir [Emoji](#emoji) plus haut), ni du JSON ni écrit par MAGI lui-même — tu remplaces le fichier entier pour le mettre à jour, rien à éditer entrée par entrée dedans. Optionnel : le sélecteur d'emoji reste juste verrouillé (avec un `Emoji: missing emoji-test.txt` explicite dans la liste principale) s'il est absent.

Un dossier `shortcuts/` à côté de l'exécutable est optionnel aussi : chaque fichier directement dedans (pas de sous-dossiers) devient une entrée lançable, listée après celles d'`apps.json` à rang de recherche égal — `.lnk`, `.bat`, `.cmd`, `.vbs`, n'importe quoi, MAGI ne vérifie pas l'extension, il passe juste le chemin au même appel `ShellExecute` que toutes les autres entrées (qui sait déjà résoudre un `.lnk` ou exécuter un `.bat`/`.vbs` exactement comme un double-clic dans l'Explorateur). Le nom de l'entrée est le nom du fichier sans son extension. Dossier absent, ou vide : aucun effet, rien de plus affiché.

### `apps.json`

```json
{
  "hotkey": "ctrl+space",
  "hotkey_enabled": true,
  "auto_restart_enabled": true,
  "copy_history_enabled": false,
  "apps": [
    { "name": "Notepad", "path": "%windir%\\system32\\notepad.exe" },
    { "name": "Command Prompt", "path": "%windir%\\system32\\cmd.exe", "cwd": "%HOMEDRIVE%%HOMEPATH%" },
    { "name": "Some background script", "path": "A:\\Scripts\\thing.ps1", "hidden": true }
  ]
}
```

- **`hotkey`** — une spec du genre `"ctrl+space"`, `"ctrl+alt+f"`, `"win+e"`, `"f14"`. Supporte `ctrl`/`control`, `alt`, `shift`, `win`/`super`, `space`, `enter`/`return`, `tab`, `esc`/`escape`, `f1`–`f24`, et les caractères seuls.
- **`hotkey_enabled`** / **`auto_restart_enabled`** / **`copy_history_enabled`** (tous optionnels ; par défaut `true`, `true`, `false` respectivement) — reflètent les trois bascules du menu tray, lues une fois au démarrage et réécrites automatiquement dès que tu en bascules une depuis le tray. Pas faits pour être édités à la main pendant que le lanceur tourne (le tray fait foi à ce moment-là), mais sûrs à définir à la main avant le premier lancement.
- **`name`** (obligatoire) — nom affiché, cherchable de façon floue.
- **`path`** (obligatoire) — un chemin simple, un chemin avec arguments (`"app.exe --flag"`), une URI shell (`ms-settings:...`, `shell:RecycleBinFolder`...), ou une [entrée spéciale `magi:`](#entrées-spéciales) ci-dessous. Tout passe par `ShellExecute`, donc tout ce qu'Explorer sait ouvrir (y compris les types de documents résolus par association de fichier, comme les fichiers `.msc`) fonctionne. Les antislashs doivent être doublés (`"A:\\Apps\\Foo.exe"`) puisque `\` est un caractère d'échappement JSON — un seul `\` est du JSON invalide et peut corrompre le chemin en silence (`\t`, `\n`... sont de vraies séquences d'échappement). Les slashs (`"A:/Apps/Foo.exe"`) marchent aussi et n'ont besoin d'aucun échappement — Windows accepte les deux.
- **`cwd`** (optionnel) — dossier de travail. Par défaut le dossier de la cible elle-même (comme un double-clic Explorer), sauf pour les entrées type `cmd.exe`/`powershell.exe` où tu voudras généralement le préciser explicitement (sinon elles démarrent dans `system32`).
- **`hidden`** (optionnel, par défaut `false`) — lance sans fenêtre visible (`SW_HIDE`), pour les scripts qui n'ont rien à afficher.

#### Entrées spéciales

| `path` | Effet |
|---|---|
| `magi:reload` | Recharge `apps.json`/`themes.json`/`notes.json`/`restart.json`/`emoji-test.txt`/`shortcuts/` sur place, sans redémarrage |
| `magi:theme-picker` | Entre dans un sélecteur de thème en direct (voir plus bas) |
| `magi:timer` | Entre dans la saisie de durée du timer ; affiche `<nom>: --:--` tant qu'il est inactif, le compte à rebours en direct une fois armé |
| `magi:notes` | Entre dans le picker Sticky Notes (voir plus haut) ; affiche `<nom>:` si vide, `<nom>: <dernière note>` sinon |
| `magi:auto-restart` | Entre dans le picker Auto-restart (voir plus haut) ; affiche `<nom>: N` pour le nombre de cibles surveillées, `0` si vide |
| `magi:copy-history` | Entre dans le picker Copy History (voir plus haut) ; affiche `<nom>: N` pour le nombre d'entrées stockées, ou `<nom>: disabled` tant que la bascule tray est désactivée (Entrée ne fait alors rien) |
| `magi:open-folder` | Ouvre le dossier contenant MAGI Launcher dans l'Explorateur — résolu à chaque lancement, suit automatiquement si le dossier est déplacé |
| `magi:empty-recycle-bin` | Affiche `<nom>: N objets, X Mo` si non vide ; `Entrée` consulte son contenu (voir [Corbeille](#corbeille) plus haut), `Shift+Entrée` ou `Suppr` la vide |
| `magi:emoji` | Entre dans le sélecteur d'emoji (voir plus haut) ; affiche `<nom>: Version X.Y` depuis `emoji-test.txt`, ou `<nom>: missing emoji-test.txt` (Entrée ne fait rien) si le fichier est absent |
| `magi:eject` | Entre dans le picker Eject (voir plus haut) |
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
  "window_size": 30,
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
- **`placeholder_text`** — affiché dans la barre de recherche quand elle est vide (remplacé par un placeholder spécifique au mode dans Timer/Sticky Notes/Auto-restart, voir leurs sections plus haut)
- **`show_clock`** — affiche l'heure actuelle (au format court de Windows de l'utilisateur) à côté de la barre de recherche
- **`window_size`** — pourcentage (0–100) de la largeur de l'écran occupée par le popup (hauteur/tailles de police suivent, en gardant un ratio 16:9, toujours centré sur le moniteur sous le curseur et jamais laissé déborder de l'écran). Ajustable en direct au clavier aussi, voir `Ctrl+1`–`Ctrl+0` plus haut — chaque pression réécrit aussitôt la valeur ici.
- **`border`** — épaisseur de bordure simulée en pixels. Ajustable en direct aussi, voir `Ctrl+-`/`Ctrl+=` plus haut.

Livré avec 100+ thèmes intégrés (surtout des palettes de personnages/jeux) dans le dict `themes` — ouvre le lanceur et sélectionne l'entrée `Themes` (`magi:theme-picker`) pour les prévisualiser et en changer en direct, sans redémarrage nécessaire. Le picker s'ouvre sur le thème actuellement actif, pas le premier par ordre alphabétique. En sélectionner un le réécrit dans `themes.json`.

## Crédits

Construit avec [Claude](https://claude.com) (l'assistant de code IA d'Anthropic).

## Licence

Copyright (C) 2026 Nyaldee. Distribué sous licence [GNU General Public License v3.0](LICENSE) — voir le fichier `LICENSE` pour le texte complet.
