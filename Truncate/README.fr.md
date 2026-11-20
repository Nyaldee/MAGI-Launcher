# Truncate

*[Read in English](README.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Donne à un fichier exactement la taille voulue : le créer, l'agrandir (rempli de zéros), le réduire, ou décaler/rogner son contenu. Pour Windows et Linux, en Rust pur, sans dépendance. Inspiré de [dbat/truncate](https://github.com/dbat/truncate), avec une syntaxe plus claire — position et options — au lieu de celle à base de signes de l'original.

## Utilisation

```
truncate <fichier> [taille[K|M|G|T]] [--skip N[K|M|G|T]] [--trim-end N[K|M|G|T]] [--quiet]
truncate <fichier>
```

Le fichier est créé s'il n'existe pas et qu'une `taille` est donnée ; rogner un fichier absent est une erreur. Les tailles sont en octets, sauf avec le suffixe K/M/G/T (binaire, ×1024, majuscules ou minuscules).

| Argument | Signification |
| --- | --- |
| `taille` | Taille finale : le fichier est créé, réduit ou agrandi à exactement cette taille. Les octets ajoutés sont des zéros. |
| `--skip N` | Octets à retirer au début (le reste est décalé au début ; tout ce qui dépasse le contenu décalé est rempli de zéros). 0 par défaut. |
| `--trim-end N` | Octets à retirer à la fin. Seulement sans `taille`. |
| `--quiet`, `-q` | N'affiche pas le résumé des tailles avant/après. |
| `--help`, `-h` | Affiche l'aide. |

Appelé avec seulement un fichier, `truncate` ne touche à rien — il affiche la taille actuelle du fichier.

## Exemples

```
truncate foo.bin                          # affiche seulement sa taille actuelle
truncate zero.bin 0                       # vide le fichier (le crée si besoin)
truncate foo.bin 2G                       # crée/redimensionne à exactement 2 Gio
truncate bar.bin --skip 19K --trim-end 19K # retire 19 Kio à chaque extrémité
truncate baz.bin 1T --skip 777            # décale le contenu depuis l'octet 777, complète de zéros jusqu'à 1 Tio
```

Chaque redimensionnement affiche un résumé, sauf avec `--quiet` :

```
> truncate foo.bin 2G
foo.bin: 10.00 MiB (10485760 B) -> 2.00 GiB (2147483648 B)
```

## Menu

`Truncate-Menu.bat` (`Truncate-Menu.sh` sous Linux) propose l'exécutable dans un menu interactif bilingue (français/anglais) : dépose un fichier dans `input\`, choisis un mode, récupère le résultat dans `output\` (l'original dans `input\` n'est jamais modifié ; afficher la taille le lit sur place, sans le copier). Pratique pour ne pas avoir à retenir la syntaxe des options.

## Linux

Tout ce qui précède fonctionne de la même façon sous Linux, avec `./truncate` et `./Truncate-Menu.sh`, et des dossiers `input/` et `output/`.

Linux a déjà sa propre commande `truncate` (GNU coreutils), avec une autre syntaxe (`truncate -s 2G fichier`). Lancez celle-ci depuis son dossier, et ne la mettez pas dans votre `PATH` devant celle du système : les scripts qui attendent la version GNU ne marcheraient plus.

## Mise à jour

Lancez `truncate_updater.bat` (`truncate_updater.sh` sous Linux) depuis le dossier `Truncate` : il télécharge la dernière version et l'extrait sur place. Vos dossiers `input` et `output` ne sont pas touchés.

## Compilation

- **Windows** — lancez `build.bat` ; l'archive se trouve ensuite dans `target\Truncate.Windows.zip`.
- **Linux** — lancez `build_linux.bat` depuis Windows (compilation dans WSL, distribution `Ubuntu`), ou `cargo build --release` sous Linux. L'archive se trouve ensuite dans `target_linux\Truncate.Linux.tar.gz`.

## Licence

Copyright (C) 2026 Nyaldee. Distribué sous licence [GNU General Public License v3.0](LICENSE) — voir le fichier `LICENSE` pour le texte complet.
