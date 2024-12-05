from scapy.all import *

def send_dhcp_request():
    """
    Sends a DHCP Discover packet to discover DHCP servers.
    """
    mac = RandMAC()
    dhcp_discover = Ether(src=mac, dst="ff:ff:ff:ff:ff:ff") / \
                    IP(src="0.0.0.0", dst="255.255.255.255") / \
                    UDP(sport=68, dport=67) / \
                    BOOTP(chaddr=mac) / \
                    DHCP(options=[("message-type", "discover"), "end"])
    
    sendp(dhcp_discover, iface="eth0", verbose=True)

def send_dhcp_decline():
    mac = RandMAC()
    fake_server_ip = "192.168.1.30"
    declined_ip = "192.168.1.100"

    dhcp_decline = Ether(src=mac, dst="ff:ff:ff:ff:ff:ff") / \
                   IP(src="0.0.0.0", dst="255.255.255.255") / \
                   UDP(sport=68, dport=67) / \
                   BOOTP(chaddr=mac) / \
                   DHCP(options=[
                       ("message-type", "decline"),
                       ("server_id", fake_server_ip),
                       ("requested_addr", declined_ip),
                       "end"
                   ])
    
    sendp(dhcp_decline, iface="eth0", verbose=True)

def send_dhcp_inform():
    mac = RandMAC()
    client_ip = "192.168.1.50"

    dhcp_inform = Ether(src=mac, dst="ff:ff:ff:ff:ff:ff") / \
                  IP(src=client_ip, dst="255.255.255.255") / \
                  UDP(sport=68, dport=67) / \
                  BOOTP(chaddr=mac, ciaddr=client_ip) / \
                  DHCP(options=[
                      ("message-type", "inform"),
                      "end"
                  ])
    
    sendp(dhcp_inform, iface="eth0", verbose=True)

def send_dhcp_message(msg_type):
    mac = RandMAC()
    dhcp_msg = Ether(src=mac, dst="ff:ff:ff:ff:ff:ff") / \
               IP(src="0.0.0.0", dst="255.255.255.255") / \
               UDP(sport=68, dport=67) / \
               BOOTP(chaddr=mac) / \
               DHCP(options=[("message-type", msg_type), "end"])
    
    sendp(dhcp_msg, iface="eth0", verbose=True)

if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage: python dhcp_sender.py <message-type>")
        print("Supported message types: discover, request, release, inform")
        sys.exit(1)

    msg_type = sys.argv[1].lower()
    if msg_type == "discover":
        send_dhcp_message("discover")
    elif msg_type == "request":
        send_dhcp_message("request")
    elif msg_type == "release":
        send_dhcp_message("release")
    elif msg_type == "decline":
        send_dhcp_decline()
    elif msg_type == "inform":
        send_dhcp_inform()
    else:
        print("Invalid message type. Supported types: discover, request, release, decline, inform")
    
    send_dhcp_message(msg_type)