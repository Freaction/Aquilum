$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'install.ps1')

function Assert-Equal {
    param(
        [object]$Actual,
        [object]$Expected
    )

    if ($Actual -cne $Expected) {
        throw "Expected '$Expected', got '$Actual'."
    }
}

function Assert-Throws {
    param([scriptblock]$Action)

    try {
        & $Action
    } catch {
        return
    }

    throw 'Expected operation to throw.'
}

$release = [pscustomobject]@{
    tag_name = 'v0.2.6'
    assets = @(
        [pscustomobject]@{ browser_download_url = 'https://github.com/Freaction/Aquilum/releases/download/v0.2.6/Aquilum_0.2.6_x64-setup.exe' }
        [pscustomobject]@{ browser_download_url = 'https://github.com/Freaction/Aquilum/releases/download/v0.2.6/Aquilum_0.2.6_aarch64-setup.exe' }
        [pscustomobject]@{ browser_download_url = 'https://github.com/Freaction/Aquilum/releases/download/v0.2.6/Aquilum_0.2.5_x64-setup.exe' }
    )
}

Assert-Equal (Get-AquilumInstallerUrl $release) $release.assets[0].browser_download_url

$missingAsset = [pscustomobject]@{ tag_name = 'v0.2.6'; assets = @() }
Assert-Throws { Get-AquilumInstallerUrl $missingAsset }
Assert-Throws { Get-AquilumInstallerUrl ([pscustomobject]@{ tag_name = '../latest'; assets = @() }) }

foreach ($url in @(
    'http://github.com/Freaction/Aquilum/releases/download/v0.2.6/Aquilum_0.2.6_x64-setup.exe',
    'https://github.com.evil.example/Freaction/Aquilum/releases/download/v0.2.6/Aquilum_0.2.6_x64-setup.exe',
    'https://github.com/Other/Aquilum/releases/download/v0.2.6/Aquilum_0.2.6_x64-setup.exe',
    'https://github.com/Freaction/Aquilum/releases/download/v0.2.5/Aquilum_0.2.6_x64-setup.exe'
)) {
    $invalidRelease = [pscustomobject]@{
        tag_name = 'v0.2.6'
        assets = @([pscustomobject]@{ browser_download_url = $url })
    }
    Assert-Throws { Get-AquilumInstallerUrl $invalidRelease }
}

Write-Output 'Windows installer tests passed.'
