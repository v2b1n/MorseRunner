"""Run original Delphi station classes without their Windows GUI/audio driver."""
from pathlib import Path
import os
import subprocess,shutil
root=Path(__file__).resolve().parents[2];src=Path(os.environ.get('MORSE_RUNNER_UPSTREAM', root.parent/'MorseRunner-original'));b=root/'target/pascal-stations';b.mkdir(parents=True,exist_ok=True)
for p in (root/'target/pascal-reference').glob('*.pas'):shutil.copy2(p,b/p.name)
for name in ['Station','DxStn','DxOper','MyStn','StnColl','QrnStn','QrmStn','Qsb']:(b/(name+'.pas')).write_text((src/(name+'.pas')).read_text().replace('SndTypes, Ini, MorseKey;', 'SndTypes, Ini, MorseKey, RndFunc;'))
(b/'Ini.pas').write_text('''unit Ini;interface
const DEFAULTRATE=11025;
type TRunMode=(rmStop,rmPileup,rmSingle,rmWpx,rmHst);
var BufSize:integer=512;RunMode:TRunMode=rmPileup;Wpm:integer=40;Lids:boolean=true;Qsb:boolean=false;Flutter:boolean=true;Call:string='N0CALL';Pitch:integer=600;Activity:integer=6;CallsFromKeyer:boolean=false;
implementation end.''')
(b/'Main.pas').write_text('''unit Main;interface
type TMainForm=class procedure Advance;end;var MainForm:TMainForm;
implementation procedure TMainForm.Advance;begin end;
initialization MainForm:=TMainForm.Create;
finalization MainForm.Free;end.''')
(b/'Log.pas').write_text('''unit Log;interface
type TQso=record Call,TrueCall:string;TrueRst,TrueNr:integer;end;
var QsoList:array of TQso;
implementation end.''')
(b/'CallLst.pas').write_text('''unit CallLst;interface
function PickCall:string;
implementation uses Classes,RndFunc;
var Calls:TStringList;
function PickCall:string;begin Result:=Calls[Random(Calls.Count)];end;
initialization Calls:=TStringList.Create;Calls.LoadFromFile('calls.txt');
finalization Calls.Free;end.''')
shutil.copy2(root/'assets/calls.txt',b/'calls.txt')
s=(src/'Contest.pas').read_text()
methods=[]
for start,end in [('function TContest.DxCount: integer;','function TContest.Minute:'),('function TContest.Minute: Single;','procedure TContest.OnMeFinishedSending;'),('procedure TContest.OnMeFinishedSending;','procedure TContest.SwapFilters;')]:methods.append(s[s.index(start,s.index('implementation')):s.index(end,s.index(start,s.index('implementation')))])
(b/'Contest.pas').write_text('''unit Contest;interface
uses MyStn,StnColl;
type TContest=class Me:TMyStation;Stations:TStations;BlockNumber:integer;
function Minute:Single;function DxCount:integer;procedure OnMeStartedSending;procedure OnMeFinishedSending;end;
var Tst:TContest;
implementation uses Station,DxStn,DxOper,Ini,RndFunc,Log;
'''+''.join(methods)+'\nend.')
(b/'stations.pas').write_text((root/'tests/pascal-reference/stations.pas').read_text())
subprocess.run(['fpc','-Mdelphi','-O2','stations.pas'],cwd=b,check=True)
with (b/'station-timeline.txt').open('w') as out:subprocess.run([str(b/'stations')],cwd=b,stdout=out,check=True)
shutil.copy2(b/'station-timeline.txt',root/'tests/fixtures/station-timeline.txt')
