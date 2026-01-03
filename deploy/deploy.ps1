ssh root@tak_server "rm -rf /root/app/docker-compose.yml /root/app/Caddyfile /root/app/.env /root/app/ory /root/app/src"
scp ./docker-compose.prod.yml root@tak_server:/root/app/docker-compose.yml
scp ./Caddyfile_prod root@tak_server:/root/app/Caddyfile
scp ./.env.prod root@tak_server:/root/app/.env
scp -r ./ory root@tak_server:/root/app
scp -r ../../playtak-ui/src root@tak_server:/root/app
ssh root@tak_server "cd /root/app && chmod -R o+rX ."