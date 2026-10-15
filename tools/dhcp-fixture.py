#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Bounded deterministic DHCP server for disposable local Ethernet fixtures only."""
import socket
import struct
import time

def checksum(data):
    data += b'\0' * (len(data) & 1)
    n=sum(struct.unpack('!%dH'%(len(data)//2),data))
    while n>>16:n=(n&65535)+(n>>16)
    return (~n)&65535

def options(data):
    result={};at=0
    while at<len(data):
        tag=data[at];at+=1
        if tag==255:return result
        if tag==0:continue
        if at>=len(data):raise ValueError('truncated option')
        size=data[at];at+=1
        if at+size>len(data) or tag in result:raise ValueError('bad/duplicate option')
        result[tag]=data[at:at+size];at+=size
    raise ValueError('missing option end')

class Server:
    def __init__(self, address='10.0.2.15', lease=3600, renew=None, rebind=None):
        self.ip=socket.inet_aton(address);self.host=socket.inet_aton('10.0.2.2')
        self.mac=bytes.fromhex('02aabbccddee');self.lease=lease
        self.renew=renew if renew is not None else lease//2
        self.rebind=rebind if rebind is not None else lease*7//8
        self.events=[];self.drop=False;self.wrong_xid=False;self.nak=False
    def handle(self,frame):
        if len(frame)>=42 and frame[12:14]==b'\x08\x06' and frame[14:22]==bytes.fromhex('0001080006040001') and frame[38:42]==self.host:
            # Router/server ARP required for unicast renewals; never claim the
            # offered guest address (ACD probes must not see a false conflict).
            return frame[22:28]+self.mac+b'\x08\x06'+bytes.fromhex('0001080006040002')+self.mac+self.host+frame[22:28]+frame[28:32]
        if len(frame)<42 or frame[12:14]!=b'\x08\x00':return None
        h=frame[14:];size=(h[0]&15)*4
        if h[0]>>4!=4 or size<20 or len(h)<size+8 or h[9]!=17:return None
        total=struct.unpack_from('!H',h,2)[0]
        if total>len(h) or total<size+8 or checksum(h[:size]):return None
        u=h[size:total]
        if struct.unpack_from('!HH',u)!=(68,67):return None
        length=struct.unpack_from('!H',u,4)[0]
        if length!=len(u) or length<248:return None
        if u[6:8]!=b'\0\0' and checksum(h[12:20]+struct.pack('!BBH',0,17,len(u))+u):return None
        p=u[8:]
        if p[:3]!=b'\x01\x01\x06' or p[236:240]!=bytes.fromhex('63825363'):return None
        try:o=options(p[240:])
        except ValueError:return None
        kind=o.get(53,b'')
        if kind not in (b'\1',b'\3'):return None
        if 54 in o and o[54]!=self.host:return None
        if 50 in o and o[50]!=self.ip:return None
        if p[12:16]!=bytes(4) and p[12:16]!=self.ip:return None
        self.events.append({'kind':kind[0],'xid':p[4:8].hex(),'ciaddr':socket.inet_ntoa(p[12:16]),'address':socket.inet_ntoa(self.ip)})
        if self.drop:return None
        reply=bytearray(240);reply[:3]=b'\x02\x01\x06';reply[4:8]=p[4:8]
        if self.wrong_xid:reply[7]^=1
        reply[10:12]=p[10:12];reply[16:20]=self.ip;reply[20:24]=self.host
        reply[28:44]=p[28:44];reply[236:240]=p[236:240]
        typ=2 if kind==b'\1' else 6 if self.nak else 5
        reply+=bytes([53,1,typ,54,4])+self.host
        if typ!=6:
            reply+=bytes([51,4])+struct.pack('!I',self.lease)
            reply+=bytes([1,4,255,255,255,0,3,4])+self.host
            reply+=bytes([58,4])+struct.pack('!I',self.renew)+bytes([59,4])+struct.pack('!I',self.rebind)
        reply+=b'\xff';reply+=bytes(max(0,300-len(reply)))
        udp=struct.pack('!HHHH',67,68,8+len(reply),0)+reply
        dst=bytes([255])*4
        c=checksum(self.host+dst+struct.pack('!BBH',0,17,len(udp))+udp) or 65535
        udp=udp[:6]+struct.pack('!H',c)+udp[8:]
        ip=struct.pack('!BBHHHBBH4s4s',0x45,0,20+len(udp),1,0,64,17,0,self.host,dst)
        ip=ip[:10]+struct.pack('!H',checksum(ip))+ip[12:]
        return bytes([255])*6+self.mac+b'\x08\x00'+ip+udp

class Stream:
    def __init__(self, peer):self.peer=peer;self.buffer=bytearray();self.received=0;self.sent=0
    def receive(self):
        while True:
            if len(self.buffer)>=4:
                n=struct.unpack_from('!I',self.buffer)[0]
                if not 14<=n<=65536:raise ValueError('frame bound')
                if len(self.buffer)>=n+4:
                    p=bytes(self.buffer[4:n+4]);del self.buffer[:n+4];self.received+=1;return p
            part=self.peer.recv(65536)
            if not part:raise RuntimeError('network disconnected')
            self.buffer+=part
    def send(self,frame):
        self.peer.sendall(struct.pack('!I',len(frame))+frame);self.sent+=1

def acquire(peer, log, process, deadline, address='10.0.2.15'):
    server=Server(address);stream=Stream(peer);peer.settimeout(.1)
    expected='NET_READY IP='+address+' UDP=12345 DHCP'
    while expected not in log.read_text():
        if time.monotonic()>deadline or process.poll() is not None:raise RuntimeError('DHCP acquisition failed: '+log.read_text())
        try:frame=stream.receive()
        except socket.timeout:continue
        reply=server.handle(frame)
        if reply:stream.send(reply)
    assert any(x['kind']==1 for x in server.events) and any(x['kind']==3 for x in server.events)
    server.stream=stream
    peer.settimeout(2)
    return server
