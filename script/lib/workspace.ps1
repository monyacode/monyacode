
function ParseMonyaCodeWorkspace {
    $metadata = cargo metadata --no-deps --offline | ConvertFrom-Json
    $env:MONYACODE_WORKSPACE = $metadata.workspace_root
    $env:RELEASE_VERSION = $metadata.packages | Where-Object { $_.name -eq "monyacode" } | Select-Object -ExpandProperty version
}
