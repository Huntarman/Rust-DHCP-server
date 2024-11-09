from scapy.all import *

def send_dhcp_packet(message_type):
    packet = (
        Ether(dst="ff:ff:ff:ff:ff:ff", src=RandMAC()) /
        IP(src="0.0.0.0", dst="255.255.255.255") /
        UDP(sport=68, dport=67) /
        BOOTP(chaddr=RandMAC()) /
        DHCP(options=[("message-type", message_type), "end"])
    )
    sendp(packet, iface="eth0", verbose=1)

send_dhcp_packet("discover")

# send_dhcp_packet("request")
# send_dhcp_packet("release")
# send_dhcp_packet("inform")
