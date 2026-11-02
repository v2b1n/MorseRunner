program stations;
uses SysUtils,Classes,Ini,RndFunc,Station,DxStn,DxOper,MyStn,StnColl,Contest,MorseKey,Log,SndTypes;
type TAccess=class(TStation);
var b,i,j:integer;discard:Single;arr:TSingleArray;chosen:string;
procedure Send(M:TStationMessage);begin Tst.Me.SendMsg(M);end;
begin
 RandomState:=77;MakeKeyer;Keyer.BufSize:=512;
 Tst:=TContest.Create;Tst.Me:=TMyStation.CreateStation;Tst.Stations:=TStations.Create;chosen:='';
 for b:=1 to 1500 do begin
  case b of
  10:Send(msgCQ);
  200:if Tst.Stations.Count>0 then begin chosen:=Tst.Stations[0].MyCall;Tst.Me.HisCall:=Copy(chosen,1,3)+'?';Send(msgHisCall);end;
  300:begin Tst.Me.HisCall:=chosen;Send(msgHisCall);Send(msgNR);end;
  400:Send(msgQm);
  500:begin Send(msgTU);Send(msgMyCall);SetLength(QsoList,1);end;
  750:Send(msgCQ);
  850:Send(msgNil);
  900:Send(msgCQ);
  end;
  Tst.BlockNumber:=b;
  if b>=6 then begin
   for i:=0 to 511 do begin discard:=3*6000*(Random-0.5);discard:=3*6000*(Random-0.5);end;
   for i:=0 to Tst.Stations.Count-1 do if Tst.Stations[i].State=stSending then arr:=Tst.Stations[i].GetBlock;
   if Tst.Me.State=stSending then arr:=Tst.Me.GetBlock;
   Tst.Me.Tick;for i:=Tst.Stations.Count-1 downto 0 do Tst.Stations[i].Tick;
  end;
  Write(b,'|',Ord(Tst.Me.State=stSending),'|');
  for i:=0 to Tst.Stations.Count-1 do if Tst.Stations[i] is TDxStation then with TDxStation(Tst.Stations[i]) do
    Write(MyCall,',',Ord(Oper.State),',',Oper.Patience,',',Ord(State),',',TAccess(Tst.Stations[i]).TimeOut,',',Wpm,',',RST,',',NR,',',Pitch,';');
  WriteLn('|rng=',RandomState);
 end;
 Tst.Stations.Free;Tst.Me.Free;Tst.Free;DestroyKeyer;
end.
