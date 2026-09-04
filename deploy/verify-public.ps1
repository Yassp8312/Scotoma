param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9.-]+$')]
    [string]$Domain
)

$ErrorActionPreference = 'Stop'
$baseUri = "https://$Domain"

function Assert-Status([string]$Path, [int]$Expected) {
    try {
        $response = Invoke-WebRequest -Uri "$baseUri$Path" -Method Get -SkipHttpErrorCheck
    } catch {
        throw "Request to $Path failed before receiving an HTTP response: $($_.Exception.Message)"
    }
    if ([int]$response.StatusCode -ne $Expected) {
        throw "$Path returned $([int]$response.StatusCode); expected $Expected"
    }
}

Assert-Status -Path '/health' -Expected 204
Assert-Status -Path '/metrics' -Expected 404
Assert-Status -Path '/not-a-public-route' -Expected 404

$socket = [Net.WebSockets.ClientWebSocket]::new()
$timeout = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
try {
    $socket.ConnectAsync([Uri]"wss://$Domain/v2/connect", $timeout.Token).GetAwaiter().GetResult()
    if ($socket.State -ne [Net.WebSockets.WebSocketState]::Open) {
        throw "WSS upgrade did not reach the open state: $($socket.State)"
    }
    $socket.CloseAsync(
        [Net.WebSockets.WebSocketCloseStatus]::NormalClosure,
        'boundary-check',
        [Threading.CancellationToken]::None
    ).GetAwaiter().GetResult()
} finally {
    $timeout.Dispose()
    $socket.Dispose()
}

Write-Output "PASS: HTTPS/WSS boundary is reachable and private routes remain closed."
