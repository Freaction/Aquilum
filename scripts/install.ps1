$ErrorActionPreference = 'Stop'

function Get-AquilumInstallerUrl {
    param([Parameter(Mandatory)][object]$Release)

    if ($Release.tag_name -notmatch '^v[0-9]+(?:\.[0-9A-Za-z-]+)+$') {
        throw 'Latest GitHub release has an invalid tag name.'
    }

    $version = $Release.tag_name.Substring(1)
    $assetName = "Aquilum_${version}_x64-setup.exe"
    $expectedUrl = "https://github.com/Freaction/Aquilum/releases/download/$($Release.tag_name)/$assetName"
    $matchingAssets = @($Release.assets | Where-Object {
        try {
            $assetUri = [Uri]$_.browser_download_url
            $assetUri.Scheme -ceq 'https' -and
                $assetUri.Host -ceq 'github.com' -and
                $assetUri.AbsolutePath -ceq "/Freaction/Aquilum/releases/download/$($Release.tag_name)/$assetName" -and
                $_.browser_download_url -ceq $expectedUrl
        } catch {
            $false
        }
    })

    if ($matchingAssets.Count -ne 1) {
        throw "Latest GitHub release does not contain one valid $assetName installer."
    }

    return $expectedUrl
}

function Invoke-AquilumInstall {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/Freaction/Aquilum/releases/latest'
    $installerUrl = Get-AquilumInstallerUrl $release
    $installerPath = Join-Path ([IO.Path]::GetTempPath()) "Aquilum-$([Guid]::NewGuid().ToString('N')).exe"

    try {
        & curl.exe -fL --proto '=https' --proto-redir '=https' --tlsv1.2 $installerUrl -o $installerPath
        if ($LASTEXITCODE -ne 0) {
            throw "Could not download Aquilum installer (curl exit code $LASTEXITCODE)."
        }
        $process = Start-Process -FilePath $installerPath -ArgumentList '/S' -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            throw "Aquilum installer failed with exit code $($process.ExitCode)."
        }

        Write-Output 'Aquilum installed successfully for the current user.'
    } finally {
        Remove-Item -LiteralPath $installerPath -Force -ErrorAction SilentlyContinue
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    Invoke-AquilumInstall
}
