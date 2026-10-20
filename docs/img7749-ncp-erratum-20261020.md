# IMG7749 HCI completion erratum — 2026-10-20

## English

The owner's IMG7749 worksheet preserves the original video wording and its earlier interpretation. HCI **Number Of Completed Packets** reports host-to-controller ACL buffer completion; it is not an acknowledgement from the Xbox or its link layer. IMG7749 establishes that the local controller reported one completed ACL packet and that no SMP Pairing Response was captured. It does not prove the peer received the request or locate the cause of silence. See the [committed LE PB correction](bluetooth-le-acl-pb-fix-20261019.md). Bluetooth work remains paused.

## 日本語

所有者のIMG7749観測文書には原動画の表記と当時の解釈を保持します。HCI **Number Of Completed Packets**はhost→controllerのACL buffer完了通知で、Xboxやlink layerのACKではありません。IMG7749で確実なのはlocal controllerの完了通知1件とSMP Pairing Response未採取です。相手への到達や無応答の原因は証明できません。[LE PB訂正](bluetooth-le-acl-pb-fix-20261019.md)を参照。Bluetooth作業は保留中です。
