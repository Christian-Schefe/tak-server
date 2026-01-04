ssh root@tak_server "rm -rf /root/app/docker-compose.yml /root/app/Caddyfile /root/app/.env /root/app/ory /root/app/src"
scp ./deploy/docker-compose.yml root@tak_server:/root/app/docker-compose.yml
scp ./deploy/Caddyfile_prod root@tak_server:/root/app/Caddyfile
scp ./deploy/.env.prod root@tak_server:/root/app/.env
scp -r ./deploy/ory root@tak_server:/root/app
scp -r ../playtak-ui/src root@tak_server:/root/app
scp ./deploy/artifacts/tak_server_latest.tar root@tak_server:/root/app/tak_server_latest.tar
ssh root@tak_server "cd /root/app && chmod -R o+rX ."
ssh root@tak_server "docker load -i /root/app/tak_server_latest.tar"