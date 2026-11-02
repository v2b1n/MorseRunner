"""Run upstream Contest.GetAudio with harmless GUI/file-sink stubs."""
from pathlib import Path
import os
import subprocess,shutil,sys
alternate='--qsk-off' in sys.argv
root=Path(__file__).resolve().parents[2];src=Path(os.environ.get('MORSE_RUNNER_UPSTREAM', root.parent/'MorseRunner-original'));b=root/('target/pascal-audio-off' if alternate else 'target/pascal-audio');b.mkdir(parents=True,exist_ok=True)
for p in (root/'target/pascal-stations').glob('*.pas'):shutil.copy2(p,b/p.name)
shutil.copy2(root/'assets/calls.txt',b/'calls.txt')
for name in ['Contest']:(b/(name+'.pas')).write_text((src/(name+'.pas')).read_text())
for name in ['MovAvg','VolumCtl','Mixers']:shutil.copy2(root/'target/pascal-reference'/(name+'.pas'),b/(name+'.pas'))
s=(b/'Ini.pas').read_text().replace('implementation end.', 'var Bandwidth:integer=500;Rit:integer=0;Qrn:boolean=true;Qrm:boolean=true;Qsk:boolean=true;Duration:integer=10000;\nimplementation end.').replace('Qsb:boolean=false','Qsb:boolean=true');
if alternate:s=s.replace('Bandwidth:integer=500','Bandwidth:integer=200').replace('Rit:integer=0','Rit:integer=-90').replace('Pitch:integer=600','Pitch:integer=800').replace('Qsk:boolean=true','Qsk:boolean=false')
(b/'Ini.pas').write_text(s)
(b/'Main.pas').write_text('''unit Main;interface
uses Classes,Ini;
type TPanel=class Caption:string;end;TSlider=class Value:Single;end;
TWave=class IsOpen:boolean;procedure WriteFrom(A,B:Pointer;N:integer);end;
TRich=class Lines:TStringList;end;
TMainForm=class
 Panel2,Panel4:TPanel;VolumeSlider1:TSlider;AlWavFile1:TWave;RichEdit1:TRich;
 procedure Advance;procedure Run(M:TRunMode);procedure PopupScoreHst;procedure PopupScoreWpx;
end;
var MainForm:TMainForm;
implementation
procedure TWave.WriteFrom(A,B:Pointer;N:integer);begin end;
procedure TMainForm.Advance;begin end;
procedure TMainForm.Run(M:TRunMode);begin end;
procedure TMainForm.PopupScoreHst;begin end;
procedure TMainForm.PopupScoreWpx;begin end;
initialization
 MainForm:=TMainForm.Create;MainForm.Panel2:=TPanel.Create;MainForm.Panel4:=TPanel.Create;
 MainForm.VolumeSlider1:=TSlider.Create;MainForm.VolumeSlider1.Value:=0.75;
 MainForm.AlWavFile1:=TWave.Create;MainForm.RichEdit1:=TRich.Create;MainForm.RichEdit1.Lines:=TStringList.Create;
finalization
 MainForm.Panel2.Free;MainForm.Panel4.Free;MainForm.VolumeSlider1.Free;MainForm.AlWavFile1.Free;
 MainForm.RichEdit1.Lines.Free;MainForm.RichEdit1.Free;MainForm.Free;
end.''')
s=(b/'Log.pas').read_text().replace('implementation end.', '''procedure CheckErr;procedure LastQsoToScreen;procedure UpdateStats;procedure UpdateStatsHst;procedure ShowRate;
implementation
procedure CheckErr;begin end;
procedure LastQsoToScreen;begin end;
procedure UpdateStats;begin end;
procedure UpdateStatsHst;begin end;
procedure ShowRate;begin end;
end.''');(b/'Log.pas').write_text(s)
s=(root/'tests/pascal-reference/stations.pas').read_text().replace('program stations;','program audio_fixture;').replace('chosen:string;','chosen:string;output:file;')
s=s.replace('Tst:=TContest.Create;Tst.Me:=TMyStation.CreateStation;Tst.Stations:=TStations.Create;chosen:=\'\';', '''Tst:=TContest.Create;Tst.Filt2.Points:=Tst.Filt.Points;chosen:='';
 AssignFile(output,'full-audio.f32');Rewrite(output,1);''')
s=s.replace('  200:if', '  100:Tst.Stations.AddQrm;\n  200:if')
a=s.index('  Tst.BlockNumber:=b;');end=s.index("  Write(b,'|'",a)
s=s[:a]+'''  arr:=Tst.GetAudio;
  if Length(arr)=1 then begin SetLength(arr,512);FillChar(arr[0],512*4,0);end;
  BlockWrite(output,arr[0],Length(arr)*4);
'''+s[end:]
s=s.replace(' Tst.Stations.Free;Tst.Me.Free;Tst.Free;DestroyKeyer;',' CloseFile(output);Tst.Free;DestroyKeyer;')
if alternate:s=s.replace('to 1500 do','to 800 do')
(b/'audio_fixture.pas').write_text(s)
subprocess.run(['fpc','-Mdelphi','-O2','audio_fixture.pas'],cwd=b,check=True)
with (b/'audio-timeline.txt').open('w') as out:subprocess.run([str(b/'audio_fixture')],cwd=b,stdout=out,check=True)
for name in ['full-audio.f32','audio-timeline.txt']:
 target=name.replace('.f32','-qsk-off.f32').replace('.txt','-qsk-off.txt') if alternate else name
 shutil.copy2(b/name,root/'tests/fixtures'/target)
