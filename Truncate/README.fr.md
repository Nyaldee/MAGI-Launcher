# Truncate

*[Read in English](README.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Donne à un fichier exactement la taille voulue : le créer, l'agrandir (rempli de zéros), le réduire, ou décaler/rogner son contenu. Rust pur, sans dépendance. Inspiré de [dbat/truncate](https://github.com/dbat/truncate), avec une syntaxe plus claire — position et options — au lieu de celle à base de signes de l'original.

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

`Truncate-Menu.bat` propose l'exe dans un menu interactif bilingue (français/anglais) : dépose un fichier dans `input\`, choisis un mode, récupère le résultat dans `output\` (l'original dans `input\` n'est jamais modifié ; afficher la taille le lit sur place, sans le copier). Pratique pour ne pas avoir à retenir la syntaxe des options.

## Mise à jour

Lancez `truncate_updater.bat` depuis le dossier `Truncate` : il télécharge la dernière version et l'extrait sur place. Vos dossiers `input\` et `output\` ne sont pas touchés.

## Licence

Gratuit pour un usage personnel et non commercial. Copyright (C) 2026 Nyaldee, tous droits réservés — voir [LICENSE](../LICENSE) pour les conditions complètes. À télécharger uniquement depuis les [releases](https://github.com/Nyaldee/nyaruhodo/releases) de ce dépôt.
