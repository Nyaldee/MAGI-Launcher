# Capture chaque fichier d'un dossier (raccourcis .lnk, .bat, .cmd, .vbs,
# n'importe quoi, sans recursion) dans un JSON au format apps.json de MAGI
# Launcher -- outil independant, ne lit/n'ecrit jamais apps.json lui-meme.
#
# Usage :
#   .\shortcuts_to_json.ps1
#       -> capture .\shortcuts\ (a cote de ce script), ecrit shortcuts_backup.json a cote du script
#   .\shortcuts_to_json.ps1 -SourceFolder "C:\Raccourcis" -OutputFile "C:\backup.json" -IncludeCwd 0
#
# Un .lnk n'est jamais capture tel quel : sa CIBLE (TargetPath, plus
# Arguments/WorkingDirectory) est resolue via WScript.Shell (COM, la meme
# API que l'Explorateur) et ecrite dans "path"/"cwd". Un .bat/.cmd/.vbs n'a
# pas cette notion de cible, son propre chemin est deja la cible.

param(
    # Sous-dossier "shortcuts" a cote du script par defaut (meme convention
    # que le dossier shortcuts/ de MAGI Launcher) -- jamais le dossier du
    # script lui-meme, pour ne jamais capturer ce script ni sa sortie.
    [string]$SourceFolder = (Join-Path $PSScriptRoot "shortcuts"),
    [string]$OutputFile = (Join-Path $PSScriptRoot "shortcuts_backup.json"),
    # 0/1, pas [switch]/[bool] : appele depuis un .bat, qui passerait le
    # texte litteral "$true"/"$false" (syntaxe PowerShell qu'il ne comprend
    # pas) -- PowerShell convertirait alors LES DEUX en $true (chaine non
    # vide = vraie). 0/1 leve l'ambiguite des deux cotes de la frontiere.
    [ValidateSet(0, 1)]
    [int]$IncludeCwd = 1
)

# Reconvertit un chemin resolu litteral ("C:\Users\Nya\AppData") vers
# "%VARIABLE%" (portable d'une machine/d'un compte a l'autre), meme
# convention tout-majuscule que apps.json. Du plus specifique au plus
# general : %LOCALAPPDATA% est un sous-chemin de %USERPROFILE%, doit donc
# etre teste avant lui sous peine de perdre la version la plus precise.
$envMap = @(
    @{ Name = "LOCALAPPDATA"; Value = $env:LOCALAPPDATA }
    @{ Name = "APPDATA"; Value = $env:APPDATA }
    @{ Name = "PROGRAMDATA"; Value = $env:ProgramData }
    @{ Name = "PROGRAMFILES(X86)"; Value = ${env:ProgramFiles(x86)} }
    @{ Name = "PROGRAMFILES"; Value = $env:ProgramFiles }
    @{ Name = "USERPROFILE"; Value = $env:USERPROFILE }
    @{ Name = "WINDIR"; Value = $env:windir }
) | Where-Object { $_.Value } | Sort-Object { $_.Value.Length } -Descending

function Compress-EnvPath {
    param([string]$Path)
    foreach ($entry in $envMap) {
        if ($Path.StartsWith($entry.Value, [StringComparison]::OrdinalIgnoreCase)) {
            return "%$($entry.Name)%" + $Path.Substring($entry.Value.Length)
        }
    }
    $Path
}

function Resolve-Entry {
    param([System.IO.FileInfo]$File, [System.__ComObject]$Shell, [System.__ComObject]$ShellApp)

    $name = [System.IO.Path]::GetFileNameWithoutExtension($File.Name)

    if ($File.Extension -ine ".lnk") {
        return [PSCustomObject]@{ name = $name; path = (Compress-EnvPath $File.FullName); cwd = $null }
    }

    # Lit un .lnk EXISTANT (pas de .Save() -- rien n'est modifie), meme API
    # que l'Explorateur pour resoudre un raccourci.
    $link = $Shell.CreateShortcut($File.FullName)
    $target = $link.TargetPath
    if ([string]::IsNullOrWhiteSpace($target)) {
        # TargetPath vide chez WScript.Shell -- soit un raccourci vraiment
        # casse (cible supprimee), soit un raccourci vers un DOSSIER SPECIAL
        # du shell (Corbeille, Panneau de configuration...) que cette API ne
        # sait pas resoudre (limitation connue de WshShortcut, pas une cible
        # manquante). Shell.Application (FolderItem.GetLink), une API
        # differente, sait lire ce cas precis -- tente ce repli AVANT de
        # conclure a un raccourci casse.
        # .GetLink.Path (le pendant de TargetPath cote Shell.Application)
        # est TOUT AUSSI vide pour ce cas -- c'est .GetLink.Target (le
        # FolderItem RESOLU), un niveau plus loin, dont le .Path porte
        # enfin le CLSID (verifie empiriquement sur un raccourci Corbeille).
        $folderItem = $ShellApp.Namespace($File.DirectoryName).ParseName($File.Name)
        $specialTarget = $folderItem.GetLink.Target.Path
        if (-not [string]::IsNullOrWhiteSpace($specialTarget) -and $specialTarget.StartsWith("::")) {
            # Meme convention que les dossiers speciaux deja presents dans
            # apps.json (Corbeille, Historique...) : explorer.exe + shell:
            # <CLSID>, la seule forme que core::launch::launch
            # (ShellExecuteExW) sait ouvrir pour ce genre de cible.
            return [PSCustomObject]@{ name = $name; path = "explorer.exe shell:$specialTarget"; cwd = $null }
        }
        # Repli reel : raccourci casse (cible supprimee) -- le .lnk lui-meme
        # plutot qu'une entree vide inutilisable.
        return [PSCustomObject]@{ name = $name; path = (Compress-EnvPath $File.FullName); cwd = $null }
    }
    $target = Compress-EnvPath $target

    # Guillemets autour de la cible SEULEMENT s'il y a des arguments --
    # sinon "path" cesse d'etre un chemin de fichier direct pour MAGI (voir
    # core::launch::resolve_target : existe tel quel -> pris directement,
    # sinon decoupe a la CommandLineToArgvW, qui exige des guillemets des
    # qu'il y a un espace dans l'executable). Meme convention que l'entree
    # "Windows Defender Quick Scan" deja dans apps.json.
    $path = if ($link.Arguments) { '"' + $target + '" ' + $link.Arguments } else { $target }
    $cwd = if ($IncludeCwd -and $link.WorkingDirectory) { Compress-EnvPath $link.WorkingDirectory } else { $null }

    [PSCustomObject]@{ name = $name; path = $path; cwd = $cwd }
}

$shell = New-Object -ComObject WScript.Shell
# Repli pour les raccourcis vers un dossier special du shell (voir
# Resolve-Entry) -- une API COM distincte de WScript.Shell, qui seule sait
# lire ce cas precis.
$shellApp = New-Object -ComObject Shell.Application
# Dossier "shortcuts" absent -> liste vide, silencieux (meme logique que le
# vrai dossier shortcuts/ de MAGI Launcher : optionnel, pas une erreur).
#
# Trie sur "name" APRES resolution (pas sur le nom de fichier brut) : les
# deux ne donnent pas toujours le meme ordre une fois l'extension retiree
# (ex: "Alpha.bat" et "Alpha B.lnk" s'inversent) -- insensible a la casse,
# meme regle que le dossier shortcuts/ de MAGI.
$entries = @(
    Get-ChildItem -Path $SourceFolder -File -ErrorAction SilentlyContinue |
        # Fichiers commencant par "." ignores -- convention pour une note/
        # placeholder (ex: ".put the shortcuts here", qui garde le dossier
        # non-vide pour Git, voir push-all-projects.bat) plutot qu'un vrai
        # raccourci a capturer.
        Where-Object { -not $_.Name.StartsWith(".") } |
        ForEach-Object { Resolve-Entry -File $_ -Shell $shell -ShellApp $shellApp } |
        Sort-Object { $_.name.ToLowerInvariant() }
)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shellApp)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell)

# JSON ecrit a la main (pas ConvertTo-Json, dont le formatage par defaut --
# indentation profonde, double espace apres chaque ":" en PowerShell 5.1 --
# ne ressemble a rien de ce que MAGI/Ports Launcher produisent) pour matcher
# exactement le style de apps.json.
function ConvertTo-JsonString {
    param([string]$Value)
    $sb = [System.Text.StringBuilder]::new()
    [void]$sb.Append('"')
    foreach ($ch in $Value.ToCharArray()) {
        switch ($ch) {
            '"' { [void]$sb.Append('\"') }
            '\' { [void]$sb.Append('\\') }
            "`t" { [void]$sb.Append('\t') }
            "`n" { [void]$sb.Append('\n') }
            "`r" { [void]$sb.Append('\r') }
            default {
                if ([int]$ch -lt 0x20) {
                    [void]$sb.Append([string]::Format('\u{0:x4}', [int]$ch))
                } else {
                    [void]$sb.Append($ch)
                }
            }
        }
    }
    [void]$sb.Append('"')
    $sb.ToString()
}

if ($entries.Count -eq 0) {
    $json = "{`n  ""apps"": []`n}"
} else {
    $blocks = foreach ($e in $entries) {
        $fields = @('      "name": ' + (ConvertTo-JsonString $e.name))
        $fields += '      "path": ' + (ConvertTo-JsonString $e.path)
        if ($e.cwd) {
            $fields += '      "cwd": ' + (ConvertTo-JsonString $e.cwd)
        }
        "    {`n" + ($fields -join ",`n") + "`n    }"
    }
    $json = "{`n  ""apps"": [`n" + ($blocks -join ",`n") + "`n  ]`n}"
}

# UTF8Encoding($false) : Set-Content -Encoding UTF8 ecrit un BOM en
# PowerShell 5.1 -- apps.json n'en a pas, et un parseur JSON strict (dont
# celui de MAGI Launcher, ecrit a la main) peut le refuser.
[System.IO.File]::WriteAllText($OutputFile, $json, (New-Object System.Text.UTF8Encoding $false))

Write-Host "Capture terminee : $($entries.Count) entree(s) ecrite(s) dans $OutputFile"
