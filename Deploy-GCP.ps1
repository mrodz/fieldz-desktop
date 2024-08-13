docker.exe compose build

if (!$?) {
	exit 1
}

if (Test-Path -Path .env -PathType Leaf) {
	Write-Host "Using environment variables in .env file"
	Get-Content .env | ForEach-Object {
		if ($_) {
			$name, $value = $_.split('=')
			Set-Content env:\$name $value
		}
	}
} else {
	Write-Host "Assuming `"GCP_APP_REPO_ID`" is an environment variable"
}

$taggedResource = "us-west2-docker.pkg.dev/$env:GCP_APP_REPO_ID/mega-scheduler-grpc-server/grpc_server_image:latest"

Write-Host "Attempting to upload artifact $taggedResource to GCP Artifact Registry"

docker.exe push $taggedResource