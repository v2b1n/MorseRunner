"""Build upstream algorithm units with empty GUI stubs, never modifying upstream.
Run from project root: python3 tests/pascal-reference/generate.py
"""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
source=Path(os.environ.get('MORSE_RUNNER_UPSTREAM', root.parent/'MorseRunner-original'))
build=root/'target/pascal-reference';build.mkdir(parents=True,exist_ok=True)
for name in ['Windows','Messages','Graphics','Controls','Forms','Dialogs']:
 (build/(name+'.pas')).write_text(f'unit {name}; interface implementation end.\n')
(build/'SndTypes.pas').write_text('''unit SndTypes; interface
const TWO_PI=2*Pi;
type TSingleArray=array of Single; TComplex=record Re,Im:Single;end;
TComplexArray=array of TComplex; TReImArrays=record Re,Im:TSingleArray;end;
procedure SetLengthReIm(var A:TReImArrays;N:integer);procedure ClearReIm(var A:TReImArrays);
implementation
procedure SetLengthReIm(var A:TReImArrays;N:integer);begin SetLength(A.Re,N);SetLength(A.Im,N);end;
procedure ClearReIm(var A:TReImArrays);begin A.Re:=nil;A.Im:=nil;end;
end.''')
(build/'Ini.pas').write_text('''unit Ini; interface
const DEFAULTRATE=11025;
type TRunMode=(rmStop,rmPileup,rmSingle,rmWpx,rmHst);
var BufSize:integer=512; RunMode:TRunMode=rmPileup;Wpm:integer=30;Lids:boolean=false;
implementation end.''')
for name in ['QuickAvg','MovAvg','VolumCtl','MorseTbl','MorseKey','Mixers']:
 s=(source/'VCL'/(name+'.pas')).read_text()
 # 32-bit pointer initialization idiom; dynamic arrays start nil on modern FPC.
 s=s.replace('Integer(Result) := 0;', 'Result := nil;')
 (build/(name+'.pas')).write_text(s)
(build/'fixture.pas').write_text((root/'tests/pascal-reference/fixture.pas').read_text())
(build/'Station.pas').write_text("""unit Station; interface
const NEVER=MaxInt;
type TStationMessage=(msgNone,msgCQ,msgNR,msgTU,msgMyCall,msgHisCall,msgB4,msgQm,msgNil,msgGarbage,msgR_NR,msgR_NR2,msgDeMyCall1,msgDeMyCall2,msgDeMyCallNr1,msgDeMyCallNr2,msgNrQm,msgLongCQ,msgMyCallNr2,msgQrl,msgQrl2,msqQsy,msgAgn);
TStationMessages=set of TStationMessage;
implementation end.""")
(build/'Contest.pas').write_text("""unit Contest;interface
type TMe=class HisCall:string;end; TContest=class Me:TMe;Minute:Single;end;
var Tst:TContest;
implementation
initialization Tst:=TContest.Create;Tst.Me:=TMe.Create;
finalization Tst.Me.Free;Tst.Free;end.""")
(build/'Log.pas').write_text('unit Log;interface implementation end.')
for name in ['Qsb','DxOper']:
 (build/(name+'.pas')).write_text((source/(name+'.pas')).read_text())
rnd=(source/'RndFunc.pas').read_text()
rnd=rnd.replace('function RndNormal: Single;', 'var RandomState:QWord=1;\nfunction Random:Single; overload;\nfunction Random(N:integer):integer; overload;\nfunction RndNormal: Single;',1)
rnd=rnd.replace('implementation',"""implementation
function Random:Single;
var V:Single;
begin RandomState:=RandomState xor (RandomState shl 13);RandomState:=RandomState xor (RandomState shr 7);RandomState:=RandomState xor (RandomState shl 17);V:=RandomState shr 40;V:=V+0.5;Result:=V/16777216;end;
function Random(N:integer):integer;begin Result:=Trunc(Random*N);if Result>=N then Result:=N-1;end;
""",1)
(build/'RndFunc.pas').write_text(rnd)
(build/'operators.pas').write_text((root/'tests/pascal-reference/operators.pas').read_text())
subprocess.run(['fpc','-Mdelphi','-O2','operators.pas'],cwd=build,check=True)
with (build/'operator-cases.csv').open('w') as out:
 subprocess.run([str(build/'operators')],cwd=build,stdout=out,check=True)
subprocess.run(['fpc' ,'-Mdelphi','-O2','fixture.pas'],cwd=build,check=True)
subprocess.run([str(build/'fixture')],cwd=build,check=True)
