#!/bin/bash
#MENU RUSTYPROXY

PORTS_FILE="/opt/rustyproxy/ports"

#FUNÇÃO PARA ABRIR PORTAS DE UM PROXY
add_proxy_port() {
    local port=$1
    local status=${2:-"\033[1;31m@RustyProxy\033[0m"}

    if is_port_in_use $port; then
        echo "A PORTA $port JÁ ESTÁ EM USO."
        return
    fi

    local command="/opt/rustyproxy/proxy --port $port --status \"$status\""
    local service_file_path="/etc/systemd/system/proxy${port}.service"
    local service_file_content="[Unit]
Description=RustyProxy ${port}
After=network.target

[Service]
LimitNOFILE=infinity
Type=simple
ExecStart=${command}
Restart=always

[Install]
WantedBy=multi-user.target"

    echo "$service_file_content" | sudo tee "$service_file_path" > /dev/null
    sudo systemctl daemon-reload
    sudo systemctl enable "proxy${port}.service"
    sudo systemctl start "proxy${port}.service"

    #SALVAR PORTAS NO ARQUIVO COM CÓDIGO ANSI
    echo -e "$port \033[1;32m$status\033[0m" >> "$PORTS_FILE"
    echo "Porta $port ABERTA COM SUCESSO."
    clear
}

#FUNÇÃO VERIFICAR PORTAS EM USO
is_port_in_use() {
    local port=$1

    #VERIFICA CONEXÕES ESTABELECIDAS OU LISTEN
    if netstat -tuln 2>/dev/null | awk '{print $4}' | grep -q ":$port$"; then
        return 0
    elif ss -tuln 2>/dev/null | awk '{print $4}' | grep -q ":$port$"; then
        return 0
    elif lsof -i :"$port" 2>/dev/null | grep -q LISTEN; then
        return 0
    else
        return 1
    fi

}

#FUNÇÃO PARA FECHAR PORTAS DE UM PROXY
del_proxy_port() {
    local port=$1

    sudo systemctl disable "proxy${port}.service"
    sudo systemctl stop "proxy${port}.service"
    sudo rm -f "/etc/systemd/system/proxy${port}.service"
    sudo systemctl daemon-reload

    #MATAR QUALQUER PROCESSO QUE AINDA ESTEJA USANDO A PORTA
    fuser -k "$port"/tcp 2>/dev/null

    #REMOVER A PORTA DO ARQUIVO DE CONTROLE
    sed -i "/^$port /d" "$PORTS_FILE"
    echo "Porta $port FECHADA COM SUCESSO."
    clear
}

#FUNÇÃO PARA ALTERAR UM STATUS DE UM PROXY
update_proxy_status() {
    local port=$1
    local new_status=$2
    local service_file_path="/etc/systemd/system/proxy${port}.service"

    if ! is_port_in_use $port; then
        echo "A PORTA $port NÃO ESTÁ ATIVA."
        return
    fi

    if [ ! -f "$service_file_path" ]; then
        echo "ARQUIVO DE SERVIÇO PARA $port NÃO ENCONTRADO."
        return
    fi

    local new_command="/opt/rustyproxy/proxy --port $port --status \"$new_status\""
    sudo sed -i "s|^ExecStart=.*$|ExecStart=${new_command}|" "$service_file_path"

    sudo systemctl daemon-reload
    sudo systemctl restart "proxy${port}.service"

    #ATUALIZAR O ARQUIVO DE PORTAS COM CÓDIGO ANSI
    sed -i "s/^$port .*/$port \033[1;32m$status\033[0m" "$PORTS_FILE"

    echo "STATUS DA PORTA $port ATUALIZADO PARA '$new_status'."
    sleep 3
    clear
}

#FUNÇÃO PARA DESINSTALAR RUSTY PROXY
    uninstall_rustyproxy() {
    echo "DESINSTALANDO RUSTY PROXY, AGUARDE..."
    sleep 3
    clear

#REMOVER TODOS OS SERVIÇOS
    if [ -s "$PORTS_FILE" ]; then
        while read -r port; do
            del_proxy_port $port
        done < "$PORTS_FILE"
    fi
	
	#REMOVER BINÁRIOS, ARQUIVOS E DIRETÓRIOS
    sudo rm -rf /opt/rustyproxy
    sudo rm -f "$PORTS_FILE"

    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    echo -e "\033[40;1;37m           RUSTY PROXY DESINSTALADO COM SUCESSO.          \E[0m"
    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    sleep 4
    clear
}

#FUNÇÃO PARA REINICIAR TODAS AS PORTAS PROXYS ABERTAS
restart_all_proxies() {
    if [ ! -s "$PORTS_FILE" ]; then
        echo "NENHUMA PORTA ENCONTRADA PARA REINICIAR."
        return
    fi

    echo "REINICIANDO TODAS AS PORTAS DO PROXY..."
    while read -r line; do
        port=$(echo "$line" | awk '{print $1}')
        status=$(echo "$line" | cut -d' ' -f2-)
        del_proxy_port "$port"
        add_proxy_port "$port" "$status"
    done < "$PORTS_FILE"

    echo "✅ TODAS AS PORTAS FORAM REINICIADAS COM SUCESSO."
    sleep 3
    clear
}

# FUNÇÃO SSLH
fun_sslh() {
		[[ "$(netstat -nltp | grep 'sslh' | wc -l)" = '0' ]] && {
			clear
			echo -e "\E[44;1;37m             INSTALADOR SSLH               \E[0m\n"
			echo -e "\n\033[1;33mVC ESTA PRESTES A INSTALAR SSLH !\033[0m\n"
			echo -ne "\033[1;32mDESEJA CONTINUAR \033[1;31m? \033[1;33m[s/n]:\033[1;37m "
			read resposta
			[[ "$resposta" = 's' ]] && {
				echo -e "\n\033[1;33mDEFINA UMA PORTA PARA SSLH !\033[0m\n"
				echo -ne "\033[1;32mQUAL A PORTA \033[1;33m?\033[1;37m "
				read porta
				[[ -z "$porta" ]] && {
					echo -e "\n\033[1;31mPorta invalida!"
					sleep 3
					clear
					fun_conexao
				}
				verif_ptrs $porta
				echo -e "\n\033[1;32mINSTALANDO SSLH AGUARDE...\033[0m"
				echo ""
				fun_instsslh() {
					[[ -e "/etc/stunnel/stunnel.conf" ]] && ptssl="$(netstat -nplt | grep 'stunnel' | awk {'print $4'} | cut -d: -f2 | xargs)" || ptssl='3128'
					[[ -e "/etc/openvpn/server.conf" ]] && ptvpn="$(netstat -nplt | grep 'openvpn' | awk {'print $4'} | cut -d: -f2 | xargs)" || ptvpn='1194'
					DEBIAN_FRONTEND=noninteractive apt-get -y install sslh
					echo -e "#Modo autónomo\n\nRUN=yes\n\nDAEMON=/usr/sbin/sslh\n\nDAEMON_OPTS='--user sslh --listen 0.0.0.0:$porta --ssh 127.0.0.1:22 --ssl 127.0.0.1:$ptssl --http 127.0.0.1:80 --openvpn 127.0.0.1:$ptvpn --pidfile /var/run/sslh/sslh.pid'" >/etc/default/sslh
					/etc/init.d/sslh start && service sslh start
				}
				echo ""
				fun_bar 'fun_instsslh'
				echo -e "\n\033[1;32mINICIANDO O SSLH !\033[0m\n"
				fun_bar '/etc/init.d/sslh restart && service sslh restart'
				[[ $(netstat -nplt | grep -w 'sslh' | wc -l) != '0' ]] && echo -e "\n\033[1;32mINSTALADO COM SUCESSO !\033[0m" || echo -e "\n\033[1;31mERRO INESPERADO !\033[0m"
				sleep 3
				fun_conexao
			} || {
				echo -e "\n\033[1;31mRetornando..."
				sleep 2
				fun_conexao
			}
		} || {
			clear
			echo -e "\E[44;1;37m             REMOVER O SSLH               \E[0m\n"
			echo -ne "\033[1;32mREALMENTE DESEJA REMOVER O SSLH \033[1;31m? \033[1;33m[s/n]:\033[1;37m "
			read respo
			[[ "$respo" = "s" ]] && {
				fun_delsslh() {
					/etc/init.d/sslh stop && service sslh stop
					apt-get remove sslh -y
					apt-get purge sslh -y
				}
				echo -e "\n\033[1;32mREMOVENDO O SSLH !\033[0m\n"
				fun_bar 'fun_delsslh'
				echo -e "\n\033[1;32mREMOVIDO COM SUCESSO !\033[0m\n"
				sleep 2
				fun_conexao
			} || {
				echo -e "\n\033[1;31mRetornando..."
				sleep 2
				fun_conexao
			}
   
#EXIBIR MENU
show_menu() {
    clear
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\033[40;1;37m                  ⚒ RUSTY PROXY MANAGER ⚒                     \E[0m"
    echo -e "\033[40;1;37m                        \033[1;32mVERSÃO: 0.2                           "
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"

   #VERIFICADOR DE PORTAS ATIVAS
    if [ ! -s "$PORTS_FILE" ]; then
        printf "NENHUMA PORTA %-34s\n" "ON"
    else
        while read -r line; do
            port=$(echo "$line" | awk '{print $1}')
            status=$(echo "$line" | cut -d' ' -f2-)
            printf " PORTA: %-5sON \033[1;31m%s\033[0m\n" "$port"
        done < "$PORTS_FILE"
    fi

    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\033[1;31m[\033[1;36m01\033[1;31m] \033[1;34m◉ \033[1;33mABRIR PORTAS \033[1;31m
[\033[1;36m02\033[1;31m] \033[1;34m◉ \033[1;33mATIVA PROXY \033[1;31m
[\033[1;36m03\033[1;31m] \033[1;34m◉ \033[1;33mDESATIVA PROXY \033[1;31m
[\033[1;36m04\033[1;31m] \033[1;34m◉ \033[1;33mALTERAR STATUS \033[1;31m
[\033[1;36m05\033[1;31m] \033[1;34m◉ \033[1;33mATIVAR SSLH \033[1;31m
[\033[1;36m06\033[1;31m] \033[1;34m◉ \033[1;33mREMOVER SCRIPT \033[1;31m
[\033[1;36m00\033[1;31m] \033[1;34m◉ \033[1;33mSAIR DO MENU \033[1;31m"
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo
    read -p "  O QUE DESEJA FAZER ?: " option

    case $option in
        1)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            read -p "DIGITE O NOME DO STATUS: " status
            add_proxy_port $port "$status"
            read -p "✅ PROXY ATIVADO COM SUCESSO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;
        2)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            del_proxy_port $port
            read -p "✅ PROXY DESATIVADO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
			clear
            ;;
			
		3)
            clear
            restart_all_proxies
            read -p "✅ PROXYS REINICIADOS. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;	
			
        4)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            read -p "DIGITE O NOVO STATUS DO PROXY: " new_status
            update_proxy_status $port "$new_status"
            read -p "✅ STATUS DO PROXY ATUALIZADO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;

      5)
	 install_sslh
            read -p "✅ SSLH CONFIGURADO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;    
            
	6)
          clear
            uninstall_rustyproxy
            read -p "◉ PRESSIONE QUALQUER TC PARA SAIR." dummy
	    clear
            exit 0
            ;;	
			
        0)
	    clear
            exit 0
            ;;
        *)
            echo "OPÇÃO INVÁLIDA. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            read -n 1 dummy
            ;;
    esac
}

#VERIFICAR SE O ARQUIVO DE PORTAS EXISTE, CASO CONTRÁRIO, CRIAR
if [ ! -f "$PORTS_FILE" ]; then
    sudo touch "$PORTS_FILE"
fi

#LOOP DO MENU
while true; do
    show_menu
done
