# Diagnóstico somente leitura. Não instala ferramentas nem altera o Windows.
$ErrorActionPreference = 'Continue'
$report = [ordered]@{
    os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    tools = [ordered]@{}
    visualStudio = @()
    windowsSdk = @()
    webView2 = @()
    scanner = 'Pendente: testar leitor fisico na interface; teclado HID generico nao identifica um scanner com seguranca.'
}

foreach ($tool in @('node', 'npm.cmd', 'rustc', 'cargo', 'rustup', 'git')) {
    $command = Get-Command $tool -ErrorAction SilentlyContinue
    if ($command) {
        $version = (& $command.Source --version 2>&1 | Out-String).Trim()
        $report.tools[$tool] = @{ path = $command.Source; version = $version; exitCode = $LASTEXITCODE }
    } else {
        $report.tools[$tool] = @{ found = $false }
    }
}

$vswherePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path -LiteralPath $vswherePath) {
    $report.visualStudio = @(& $vswherePath -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json | ConvertFrom-Json)
}
$sdkPath = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
if (Test-Path -LiteralPath $sdkPath) {
    $report.windowsSdk = @(Get-ChildItem -LiteralPath $sdkPath -Directory | Select-Object -ExpandProperty Name)
}

$clientKeys = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\*',
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\*',
    'HKCU:\Software\Microsoft\EdgeUpdate\Clients\*'
)
$report.webView2 = @(foreach ($key in $clientKeys) {
    Get-ItemProperty -Path $key -ErrorAction SilentlyContinue |
        Where-Object { $_.name -like '*WebView2*' } |
        Select-Object name, pv
})
$report | ConvertTo-Json -Depth 8
