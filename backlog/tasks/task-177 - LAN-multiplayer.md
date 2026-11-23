---
id: TASK-177
title: LAN multiplayer
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
updated_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
ordinal: 177000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GAME.EXE plays over a LAN (CLANWrapper, 0x1fc6d4-0x1fc88c; UDP transport 0x1fbff0) and Westwood Online (WOL..., chat events 0x1f3900-0x1f4174, servers gone). Game set-up changes are messages (CHANGE_MAP, CHANGE_MONEY, CHANGE_CRATES... 0x1efce8-0x1effe4), the game keeps in step by orders (CNetworkAdmin, 0x20bafb-0x20c564) and can record them (game.rec 0x20e0d4, at 0x11e636). The co-op campaign is WolCoopSetupMgr (0x1efca8). Low priority. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two copies on one network can play a skirmish together
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The original's networking functions, by the names its messages give (GAME.EXE file offset, size to the next known function, the message). Westwood Online's servers are gone, so only the LAN, co-op and in-game sync parts matter:
- CBaseChannelList::UpdateTopic: 0x7b120, 0x110: CBaseChannelList::UpdateTopic : %s
- CBaseUserList::CheckForAndRequestSquadInformation: 0x7b845, 0x6b: CBaseUserList::CheckForAndRequestSquadInformation Set Squad ID for Use
- CBaseWrapper: 0x7dd80, 0x110: CBaseWrapper:
- CBaseWrapper::CoopChangeDifficulty: 0x7c9d0, 0x60: CBaseWrapper::CoopChangeDifficulty
- CBaseWrapper::CoopChangeHouse: 0x7c970, 0x60: CBaseWrapper::CoopChangeHouse
- CBaseWrapper::CoopChangeLoad: 0x7c890, 0xd0: CBaseWrapper::CoopChangeLoad
- CBaseWrapper::CoopChangeMovies: 0x7ca30, 0x6: CBaseWrapper::CoopChangeMovies
- CBaseWrapper::CoopClientSendAccepted: 0x7c810, 0x50: CBaseWrapper::CoopClientSendAccepted
- CBaseWrapper::CoopClientSendStarted: 0x7d476, 0x22a: CBaseWrapper::CoopClientSendStarted
- CBaseWrapper::CoopOrderGameStart: 0x7c7a0, 0x6: CBaseWrapper::CoopOrderGameStart
- CBaseWrapper::ProcessChangeUpdate: 0x7cdb0, 0x1a6: CBaseWrapper::ProcessChangeUpdate
- CBlackBox::Close: 0x81a90, 0x196: CBlackBox::Close
- CBlackBox::HHF: 0x82a60, 0x120: CBlackBox::HHF Table:
- CBlackBox::Host_Receive_Details: 0x832a0, 0x8c: CBlackBox::Host_Receive_Details
- CBlackBox::Host_Receive_Mangled_Port_N: 0x82e0f, 0x28e: CBlackBox::Host_Receive_Mangled_Port_N
- CBlackBox::Host_Receive_Running: 0x82e0f, 0x28e: CBlackBox::Host_Receive_Running
- CBlackBox::Host_Send_Black_Box_State: 0x82144, 0x28f: CBlackBox::Host_Send_Black_Box_State
- CBlackBox::Host_Send_Player_List: 0x81fe0, 0x164: CBlackBox::Host_Send_Player_List
- CBlackBox::Host_Send_Unmangled_Port_N: 0x830f0, 0x90: CBlackBox::Host_Send_Unmangled_Port_N
- CBlackBox::Init: 0x81620, 0xe8: CBlackBox::Init at time %s
- CBlackBox::Manipulate_Addressing: 0x836d0, 0x90: CBlackBox::Manipulate_Addressing
- CBlackBox::Peer_Receive_Black_Box_State: 0x82c10, 0x1ff: CBlackBox::Peer_Receive_Black_Box_State
- CBlackBox::Peer_Receive_Player_List: 0x82c10, 0x1ff: CBlackBox::Peer_Receive_Player_List
- CBlackBox::Peer_Receive_Unmangled_Port_N: 0x82e0f, 0x28e: CBlackBox::Peer_Receive_Unmangled_Port_N
- CBlackBox::Peer_Send_Details: 0x83180, 0x120: CBlackBox::Peer_Send_Details
- CBlackBox::Peer_Send_Mangled_Port_N: 0x83490, 0x110: CBlackBox::Peer_Send_Mangled_Port_N
- CBlackBox::Peer_Send_Running: 0x81708, 0x28f: CBlackBox::Peer_Send_Running
- CBlackBox::State: 0x81708, 0x28f: CBlackBox::State               = %s
- CChatEventHandler::CheckAndRequestAdditionalUserInformation: 0x99fe0, 0x240: CChatEventHandler::CheckAndRequestAdditionalUserInformation Set Squad 
- CChatEventHandler::OnConnection: 0x98e40, 0x110: CChatEventHandler::OnConnection when system appears to be shutdown
- CChatEventHandler::OnSquadInfo: 0x9a4ab, 0x28a: CChatEventHandler::OnSquadInfo Set Squad ID for User %s to %d\n
- CChatEventHandler::OnUserLocale: 0x9b8fb, 0x65: CChatEventHandler::OnUserLocale Set Squad ID for User %s to %d\n
- CHandshake::StartHandshake: 0xc8920, 0xa0: CHandshake::StartHandshake(%d) at time %s.
- CHello::Close: 0xc8ee0, 0x50: CHello::Close
- CHello::Init: 0xc8e30, 0xad: CHello::Init
- CHello::IsEnabled: 0x81708, 0x28f: CHello::IsEnabled              = %d
- CHello::Receive_Hello: 0xc91f0, 0x1ee: CHello::Receive_Hello: discarding hello packet from attempt %d. This i
- CHello::Send_Hello: 0xc8ff0, 0xf0: CHello::Send_Hello: Blasting hello packet to player shortuuid %u at %s
- CLANLobby::SendPingResponse: 0xdac90, 0x140: CLANLobby::SendPingResponse : name is empty, no point sending reply
- CLANWrapper::InterpretSetupInfo: 0xdc960, 0x260: CLANWrapper::InterpretSetupInfo
- CLANWrapper::ServerLogin: 0xdc260, 0x100: CLANWrapper::ServerLogin
- CMangler::Build_Mangler_Query_Packet: 0xe5ac0, 0xc0: CMangler::Build_Mangler_Query_Packet
- CMangler::Close: 0xe5790, 0x1d0: CMangler::Close
- CMangler::Init: 0xe5790, 0x1d0: CMangler::Init(%un)
- CMangler::Interpret_Mangler_Reply_Packet: 0xe59d0, 0xf0: CMangler::Interpret_Mangler_Reply_Packet: Crc error! Packet ignored.
- CMangler::Is_Enabled: 0x81708, 0x28f: CMangler::Is_Enabled           = %d
- CMangler::Poll: 0xe59d0, 0xf0: CMangler::Poll: Non-mangler packet ignored.
- CNetUtilEventHandler::OnGameresSent: 0x107c70, 0x60: CNetUtilEventHandler::OnGameresSent SUCCESS reported\n
- CNetworkAdmin::EndGame: 0x109d20, 0x2e: CNetworkAdmin::EndGame
- CNetworkAdmin::EndUpdateLoading: 0x1090b0, 0x20: CNetworkAdmin::EndUpdateLoading
- CNetworkAdmin::SendGameReadyHandshake: 0x108aa0, 0x17f: CNetworkAdmin::SendGameReadyHandshake(%d) at time %s.
- CNetworkAdmin::Service: 0x10a236, 0x275: CNetworkAdmin::Service = Got unexpected admin message\n
- CNetworkAdmin::SetupHandshake: 0x108860, 0x14b: CNetworkAdmin::SetupHandshake(%d) at time %s.
- CNetworkAdmin::StartUpdateLoading: 0x108d10, 0x184: CNetworkAdmin::StartUpdateLoading
- CNetworkAdmin::UpdateLoadingCheckTimeValues: 0x108d10, 0x184: CNetworkAdmin::UpdateLoadingCheckTimeValues WARNING Firewall timeout c
- CPlayer::SetName: 0x11614a, 0x56: Warning: CPlayer::SetName("%s") disallowed - current name is %s.
- CPlayer::SetReceivedAddress: 0x116040, 0x50: CPlayer::SetReceivedAddress to %s for %s.
- CPlayerList::Close: 0x116510, 0x50: CPlayerList::Close
- CPlayerList::Init: 0x1164e0, 0x30: CPlayerList::Init
- CPlayerList::RemoveAll: 0x116510, 0x50: CPlayerList::RemoveAll
- CPlayerList::RemoveAllButMe: 0x116a70, 0x60: CPlayerList::RemoveAllButMe
- CPlayerList::RemovePlayer: 0x116900, 0xa0: * CPlayerList::RemovePlayer(%s) : WARNING - name operation on LAN.
- CPortUtil::Get_Free_Random_Port_H: 0x119100, 0x70: CPortUtil::Get_Free_Random_Port_H: Trying port %uh...
- CScoreQuickMatch::QueryLaunchMatchReplay: 0x12db10, 0xb0: CScoreQuickMatch::QueryLaunchMatchReplay: Replay channel seek Timed ou
- CSnmpUtil::Get_Local_Association_Ip: 0x1409a8, 0x26a: CSnmpUtil::Get_Local_Association_Ip:
- CSortList::PingList: 0x18aa90, 0xc0: CSortList::PingList - Ping List ended 
- CSortList::RefreshFromUserAndChannelList: 0x18a80a, 0x1b6: CSortList::RefreshFromUserAndChannelList Set Squad ID for User %s to %
- CUuid::Set_From_Guid: 0x174990, 0xc0: CUuid::Set_From_Guid: %s
- CWolChatRoomManager::Activate: 0x17ce33, 0x20d: CWolChatRoomManager::Activate
- CWolChatRoomManager::Animate: 0x17d7b0, 0x100: CWolChatRoomManager::Animate Refreshing WOL chat room lists
- CWolLobbyManager::Activate: 0x17f100, 0xc6: CWolLobbyManager::Activate
- CWolLoginManager::Activate: 0x186fa0, 0x209: CWolLoginManager::Activate
- CWolManager::Activate: 0x189590, 0x50: CWolManager::Activate
- CWolManager::ErrorHandler: 0x189790, 0x24c: CWolManager::ErrorHandler entered with code %d 0x%x\n
- CWolSetupManager::Activate: 0x182990, 0x70: CWolSetupManager::Activate
- CWolSetupManager::KickAllClients: 0x185310, 0x234: CWolSetupManager::KickAllClients
- CWolUI::PopUp: 0x18b8e0, 0x40: CWolUI::PopUp
- CWolWrapper::AddBuddy: 0x18f600, 0x80: CWolWrapper::AddBuddy Adding buddy
- CWolWrapper::DecodeSetupInfo: 0x18ed30, 0x1b4: CWolWrapper::DecodeSetupInfo
- CWolWrapper::DecodeSideSetupInfo: 0x18ef10, 0x230: CWolWrapper::DecodeSideSetupInfo
- CWolWrapper::InterpretSetupInfo: 0x18f180, 0x140: CWolWrapper::InterpretSetupInfo
- CWolWrapper::JoinChannel: 0x18d550, 0x50: CWolWrapper::JoinChannel failed
- CWolWrapper::JoinDefaultChatChannel: 0x18d370, 0x17c: CWolWrapper::JoinDefaultChatChannel Selected channel %s as default cha
- CWolWrapper::OpenCurrentGameChannel: 0x18d6e0, 0x68: CWolWrapper::OpenCurrentGameChannel requested\n
- CWolWrapper::ReportResults: 0x18ff80, 0x80: CWolWrapper::ReportResults requested\n
- CWolWrapper::RequestInsiderStatus: 0x18e3a0, 0x50: CWolWrapper::RequestInsiderStatus requested\n
- CWolWrapper::RequestQMGameStart: 0x18e430, 0x50: CWolWrapper::RequestQMGameStart requested\n
- CWolWrapper::RequestSquadInfo: 0x18e316, 0x8a: CWolWrapper::RequestSquadInfo requested\n
- CWolWrapper::SendSetupInfo: 0x18e8a0, 0x18b: CWolWrapper::SendSetupInfo
- CWolWrapper::ServerLogin: 0x18cf60, 0xd3: CWolWrapper::ServerLogin
- CWolWrapper::SetLocale: 0x18e3f0, 0x40: CWolWrapper::SetLocale Failed to send request
- CWolWrapper::fpSortList: 0x1917f0, 0x90: CWolWrapper::fpSortList:
- CWolWrapper::~CWolWrapper: 0x18c630, 0x91: CWolWrapper::~CWolWrapper
- GetChannel: 0x7af90, 0x50: GetChannel: num > channels
- GetChannelName: 0x7b040, 0x54: GetChannelName: num > channels
- GetSecondHumanSide: 0x7e9e0, 0x1a4: GetSecondHumanSide: There is no second human side
- GetUser: 0x7bd40, 0x25: GetUser: num > users
- GetUsersName: 0x7beb0, 0x50: GetUsersName: num > users
- JoinChannel: 0x17b320, 0xf8: JoinChannel : joining %s
- MessageManager::RemoveTarget: 0xe9340, 0x60: MessageManager::RemoveTarget - removed side %d\n
- MessageManager::Send: 0xe93a0, 0x14a: MessageManager::Send - failed to find target with uuid %s in list\n
- OnInsiderStatus: 0x9ba60, 0xb0: OnInsiderStatus : %s
- SetNewLobbyCheckBox: 0x180cf0, 0x120: SetNewLobbyCheckBox: Unknown 
- SetPing: 0x7b230, 0x90: SetPing: num<0
- TurnWolOff: 0x18b120, 0x5e: TurnWolOff()
- TurnWolOn: 0x18b460, 0x1d0: TurnWolOn()
- UpdateHandshake: 0x10937c, 0x275: UpdateHandshake() : Got unexpected msg from uuid %s\n
- WinsockInterfaceClass: 0x198200, 0x40: WinsockInterfaceClass::WriteTo discarded as unknown UUID for player
<!-- SECTION:NOTES:END -->
