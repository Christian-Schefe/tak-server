# Clean up old deployment files
ssh root@tak_server "rm -rf /root/app/docker-compose.yml /root/app/caddy /root/app/.env /root/app/ory /root/app/src /root/app/tak_server_latest.tar"

# Docker Compose file
scp ./deploy/docker-compose.yml root@tak_server:/root/app/docker-compose.yml

# Environment file
scp ./deploy/.env.production root@tak_server:/root/app/.env

# Caddy and Ory configs
scp -r ./deploy/caddy root@tak_server:/root/app
scp -r ./deploy/ory root@tak_server:/root/app

# Frontend source files
scp -r ../playtak-ui/src root@tak_server:/root/app

# Docker image
scp ./deploy/artifacts/tak_server_latest.tar root@tak_server:/root/app/tak_server_latest.tar

# Set permissions and load Docker image
ssh root@tak_server "cd /root/app && chmod -R o+rX ."
ssh root@tak_server "docker load -i /root/app/tak_server_latest.tar"