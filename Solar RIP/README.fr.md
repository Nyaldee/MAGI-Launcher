# Solar RIP

*[Read in English](README.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Un petit patcher pour le format `.rip`, pour Windows et Linux — comme Lunar IPS, mais avec des offsets et des tailles sur 64 bits au lieu des champs de 3 octets d'IPS : pas de limite de 16 Mo. Un patch marche dans les deux sens : il patche l'original, et ramène le fichier patché à l'original. Les fichiers de toute taille sont patchés en un seul passage, sans être chargés en mémoire. Voir [Linux](#linux) pour ce qui y diffère.

## Format

Un `.rip` contient la différence entre un fichier original et sa version modifiée : la signature `RIP`, puis les enregistrements, puis les tailles et CRC32 des deux fichiers. Les nombres de taille fixe sont en little-endian.

| Octets | Contenu |
|---|---|
| 3 | Signature `RIP` |
| variable | Enregistrements |
| 8 | Taille du fichier original |
| 4 | CRC32 du fichier original |
| 8 | Taille du fichier modifié |
| 4 | CRC32 du fichier modifié |
| 4 | CRC32 de tout ce qui précède |

Chaque enregistrement est un nombre donnant son écart depuis la fin de l'enregistrement précédent (depuis le début des fichiers pour le premier), un nombre donnant sa longueur (au moins 1), puis autant d'octets : l'original XOR le fichier modifié à cette position. Ces nombres utilisent l'encodage de longueur variable de BPS : 7 bits par octet, le bit fort marque le dernier octet, et chaque octet de plus décale la plage des valeurs, si bien que chaque nombre n'a qu'une seule écriture.

Les deux fichiers sont vus complétés par des zéros jusqu'à la longueur du plus long. Hors des enregistrements, ils sont identiques, et tout ce qui dépasse la fin du plus court est dans les enregistrements. Appliquer un patch combine chaque enregistrement (XOR) au fichier à sa position, ce qui change l'un des deux fichiers en l'autre, puis ajuste le résultat à la taille de l'autre fichier. Le CRC32 est celui de zip et gzip (IEEE 802.3). Les tailles et CRC32 sont à la fin pour qu'un patch s'écrive en un seul passage.

Le format est figé : il n'existe qu'en une seule version, qui ne changera pas.

## Sécurité

Comme un patch connaît les deux fichiers, chaque application est vérifiée :

- **Rien n'est remplacé avant que le résultat soit complet et vérifié.** Le nouveau fichier s'écrit à côté de celui choisi, en un seul passage, et ne prend sa place qu'ensuite. En cas d'échec, le fichier reste intact et le fichier temporaire est supprimé.
- **Un fichier déjà patché** est reconnu comme tel, et peut être ramené à l'original.
- **Un fichier qui n'est aucun des deux** est signalé avec sa taille et son CRC32, et n'est patché qu'après confirmation (`--force` en ligne de commande).
- **Un patch abîmé par accident** est refusé dès son ouverture, grâce à son propre CRC32.
- **Un patch ne peut pas faire écrire à Solar RIP plus que les fichiers et ses propres données** : un patch mal formé ou malveillant ne peut pas remplir le disque.
- **Chaque patch créé par Solar RIP est vérifié dans les deux sens avant d'être enregistré** : sans rien écrire, il doit changer l'original exactement en le fichier modifié, et le fichier modifié exactement en l'original.

Le CRC32 protège contre les accidents — un téléchargement corrompu, un mauvais fichier — pas contre une falsification : ce n'est pas une signature, et un patch peut être fabriqué pour y correspondre. N'appliquez que des patchs de sources de confiance.

## Utilisation

- **Apply Patch** — choisir un `.rip`, puis le fichier à patcher. Le fichier est patché sur place une fois le résultat vérifié (voir [Sécurité](#sécurité)). Choisir un fichier déjà patché propose de restaurer l'original.
- **Create Patch** — choisir un fichier original et sa version modifiée, puis enregistrer le `.rip` obtenu. L'application se ferme ensuite.
- **Scramble File** — enregistre une copie d'un fichier dont les 4 premiers Ko sont remplacés par du bruit, ainsi que le `.rip` qui la répare. L'original n'est jamais modifié.
- **Verify Patch** — affiche les fichiers original et modifié qu'attend un `.rip` (CRC32 et taille) et son nombre d'enregistrements, sans toucher à aucun fichier.
- **Register** — associe les fichiers `.rip` à cet exécutable (pour l'utilisateur courant, sans droits administrateur). Un double-clic sur un `.rip` passe ensuite directement au choix du fichier à patcher, même quand Solar RIP est ouvert. Repasse à OFF si l'exécutable a été déplacé : un clic met l'association à jour.
- **Create Log** — active l'écriture d'un fichier `.log` à côté du fichier patché (Apply Patch) ou du patch (Create Patch, Scramble File).
- **Exit**
- **Website** — ouvre nyaruhodo.com.

Dans les messages, Entrée valide et Échap annule. Une seule fenêtre principale tourne à la fois — un second lancement ramène la première au premier plan.

## Ligne de commande

Les mêmes opérations fonctionnent sans fenêtre, pour les scripts :

```
solar_rip apply <patch.rip> <fichier> [--force] [--log]
solar_rip restore <patch.rip> <fichier> [--log]
solar_rip create <original> <modifié> <patch.rip> [--log]
solar_rip scramble <fichier> <brouillé> <patch.rip> [--log]
solar_rip verify <patch.rip>
solar_rip --help
```

- `apply` patche le fichier sur place. Il s'arrête si le fichier est déjà patché, et s'il n'est pas l'original qu'attend le patch, sauf avec `--force`.
- `restore` ramène un fichier patché à l'original, sur place.
- `--log` écrit le même fichier `.log` que **Create Log**.
- Code de sortie : `0` en cas de succès, `1` si l'opération échoue, `2` si la commande est mal formée. Les erreurs vont sur la sortie d'erreur.

Solar RIP est un programme à fenêtre : sous Windows, une invite interactive ne l'attend pas et affiche son résultat après l'invite suivante. Les scripts (`.bat`, PowerShell) l'attendent et récupèrent sa sortie et son code de sortie ; dans un `cmd` interactif, utiliser `start /wait /b solar_rip …`.

## Linux

Tout ce qui précède fonctionne aussi sous Linux, avec ces différences :

- **Les boîtes de fichier** viennent de votre bureau, par le portail XDG (`xdg-desktop-portal`, installé avec GNOME, KDE et la plupart des bureaux) : elles ressemblent au reste du système.
- **Register** écrit un type MIME pour `*.rip`, une entrée d'application masquée et son icône dans `~/.local/share`, puis lance `update-mime-database`, `update-desktop-database` et `xdg-mime` s'ils sont installés (`shared-mime-info`, `desktop-file-utils`, `xdg-utils`). Rien n'est ajouté au menu des applications.
- **Website** ouvre l'adresse avec `xdg-open` (`xdg-utils`).
- **Wayland** ne laisse pas une application placer sa fenêtre ni se ramener au premier plan : le compositeur décide où Solar RIP s'ouvre, et un second lancement n'y ramène pas la première fenêtre.

## Licence

Gratuit pour un usage personnel et non commercial. Copyright (C) 2026 Nyaldee, tous droits réservés — voir [LICENSE](../LICENSE) pour les conditions complètes. À télécharger uniquement depuis les [releases](https://github.com/Nyaldee/nyaruhodo/releases) de ce dépôt.

[Claude](https://claude.com), l'IA d'Anthropic, a contribué au code de ce projet.
