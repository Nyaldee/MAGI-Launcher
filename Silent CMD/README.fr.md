# Silent CMD

*[Read in English](README.md) · [日本語で読む](README.jp.md) · [阅读中文版](README.zh-CN.md)*

Un petit lanceur pour Windows qui exécute une commande ou un script (`.bat`/`.cmd`) sans fenêtre de console visible.

## Utilisation

```
silent_cmd.exe [options] <commande> [arguments...]
```

Tout ce qui suit les options est transmis à `cmd /C` exactement tel qu'écrit, guillemets et caractères spéciaux (`&`, `^`, `%`…) compris. Le code de sortie de la commande et ses redirections fonctionnent normalement — seule la fenêtre de console est masquée. Les options se combinent, dans n'importe quel ordre, avant la commande.

| Option | Description |
|---|---|
| `--delay <secondes>` | Attend avant de lancer la commande. |
| `--log <fichier>` | Écrit la sortie et les erreurs de la commande dans `fichier` (écrasé à chaque lancement). Un chemin relatif part de `--dir` s'il est donné. |
| `--dir <chemin>` | Lance la commande avec `chemin` comme dossier de travail. |
| `--nowait` | Lance la commande et rend la main tout de suite au lieu d'attendre qu'elle se termine. |

```
silent_cmd.exe myscript.bat
silent_cmd.exe myscript.bat arg1 arg2
silent_cmd.exe --delay 5 myscript.bat
silent_cmd.exe --log out.log myscript.bat
silent_cmd.exe --dir "C:\Work" myscript.bat
silent_cmd.exe --nowait myscript.bat
silent_cmd.exe --delay 5 --dir "C:\Work" --log out.log myscript.bat arg1 arg2
```

Code de sortie : celui de la commande (`0` avec `--nowait` une fois lancée), `1` si elle ne peut pas être lancée ou si le fichier de `--log` ne peut pas être créé, `2` s'il n'y a pas de commande ou qu'une option est incomplète.

Lancée sans console (raccourci, Planificateur de tâches), la commande reçoit une entrée standard vide : un `pause` ou un `set /p` rend la main tout de suite au lieu d'attendre indéfiniment une console invisible. C'est toujours le `cmd.exe` du système qui est utilisé, sans les commandes AutoRun du registre (`/D`).

Un `.bat` lancé directement par double-clic fait toujours apparaître une console une fraction de seconde — c'est `cmd.exe` lui-même, pas Silent CMD. Pour un lancement vraiment invisible, la cible du double-clic doit être Silent CMD lui-même, par exemple un raccourci vers `silent_cmd.exe` avec le script en argument, jamais le `.bat` directement.

## Licence

Gratuit pour un usage personnel et non commercial. Copyright (C) 2026 Nyaldee, tous droits réservés — voir [LICENSE](../LICENSE) pour les conditions complètes. À télécharger uniquement depuis les [releases](https://github.com/Nyaldee/nyaruhodo/releases) de ce dépôt.

[Claude](https://claude.com), l'IA d'Anthropic, a contribué au code de ce projet.
