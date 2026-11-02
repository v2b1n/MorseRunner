program operators;
uses SysUtils, Math, Ini, Station, Contest, RndFunc, DxOper;
const calls:array[0..5]of string=('DL1ABC','DL1?','1AB','?','DL1ABX','W9XYZ');
 masks:array[0..15]of integer=(1,2,4,8,16,32,64,128,256,512,36,40,12,24,6,34);
var op:TDxOperator; st,ci,li,si,seed,mi,i:integer;msgs:TStationMessages;reply:TStationMessage;delay,timeout:integer;
begin
 Lids:=false;Wpm:=30;
 for seed:=1 to 8 do for st:=0 to 7 do for mi:=0 to 15 do for ci:=0 to 5 do for li:=0 to 1 do for si:=0 to 1 do begin
  RandomState:=seed;Lids:=li=1;if si=1 then RunMode:=rmSingle else RunMode:=rmPileup;
  op:=TDxOperator.Create;op.Call:='DL1ABC';op.Skills:=2;op.Patience:=5;op.RepeatCnt:=1;op.State:=TOperatorState(st);Tst.Me.HisCall:=calls[ci];
  msgs:=[];for i:=0 to 22 do if (masks[mi] and (1 shl i))<>0 then Include(msgs,TStationMessage(i));
  op.MsgReceived(msgs);reply:=op.GetReply;delay:=op.GetSendDelay;timeout:=op.GetReplyTimeout;
  WriteLn(seed,',',st,',',mi,',',ci,',',li,',',si,',',Ord(op.State),',',op.Patience,',',op.RepeatCnt,',',Ord(reply),',',delay,',',timeout);
  op.Free;
 end;
end.
