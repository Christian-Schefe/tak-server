# Clean up old deployment files
ssh root@tak_server "rm -rf /root/app/*"

# Copy frontend dist to artifacts
Remove-Item -Path ./deploy/artifacts/dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -Path ./deploy/artifacts/dist -ItemType Directory | Out-Null
Copy-Item -Path ../tak-frontend/dist/tak-frontend/browser/* -Destination ./deploy/artifacts/dist -Recurse -Force

# Docker Compose file
scp ./deploy/docker-compose.yml root@tak_server:/root/app/docker-compose.yml


# Environment file
scp ./deploy/.env.production root@tak_server:/root/app/.env

# TLS Certificates
scp -r ./deploy/certs root@tak_server:/root/app

# Caddy and Ory configs
scp -r ./deploy/caddy root@tak_server:/root/app
scp -r ./deploy/ory root@tak_server:/root/app

# Frontend source files
scp -r ./deploy/artifacts/dist root@tak_server:/root/app

# Docker image
scp ./deploy/artifacts/tak_server_latest.tar root@tak_server:/root/app/tak_server_latest.tar

# Set permissions and load Docker image
ssh root@tak_server "cd /root/app && chmod -R o+rX ."
ssh root@tak_server "docker load -i /root/app/tak_server_latest.tar"