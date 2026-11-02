program fixture;
uses SysUtils, Math, Classes, SndTypes, Ini, MorseKey, QuickAvg, MovAvg, VolumCtl, Mixers, Qsb, RndFunc;
var k:TKeyer; a,b:TSingleArray; qa:TQuickAverage; ma:TMovingAverage; agc:TVolumeControl;
    modul:TModulator;fade:TQsb; iq:TReImArrays; i,j:integer; f:file; name:string;
procedure Save(const N:string;const A:TSingleArray);
begin AssignFile(f,N);Rewrite(f,1);if Length(A)>0 then BlockWrite(f,A[0],Length(A)*4);CloseFile(f);end;
begin
 k:=TKeyer.Create;k.BufSize:=512;
 k.Wpm:=20;k.MorseMsg:=k.Encode('E');a:=k.Envelope;Save('key-e.f32',a);
 k.Wpm:=30;k.MorseMsg:=k.Encode('CQ DL1ABC TEST');a:=k.Envelope;Save('key-cq.f32',a);
 k.Wpm:=80;k.MorseMsg:=k.Encode('5NN TT1 EEEEE 021');a:=k.Envelope;Save('key-error.f32',a);k.Free;
 qa:=TQuickAverage.Create(nil);qa.Passes:=3;qa.Points:=7;SetLength(a,1024);
 for i:=0 to High(a) do a[i]:=qa.Filter(Sin(i*0.173)+Cos(i*0.071));Save('quick-average.f32',a);qa.Free;
 ma:=TMovingAverage.Create(nil);ma.Passes:=3;ma.Points:=15;ma.SamplesInInput:=512;ma.GainDb:=0;
 SetLength(a,512);SetLength(b,2048);
 for j:=0 to 3 do begin for i:=0 to 511 do a[i]:=Sin((j*512+i)*0.073)+Cos((j*512+i)*0.033);a:=ma.Filter(a);Move(a[0],b[j*512],512*4);end;
 Save('moving-average.f32',b);ma.Free;
 agc:=TVolumeControl.Create(nil);agc.NoiseInDb:=76;agc.NoiseOutDb:=76;agc.AttackSamples:=155;agc.HoldSamples:=155;agc.AgcEnabled:=true;
 SetLength(a,4096);for i:=0 to High(a) do begin a[i]:=Sin(i*0.37)*2000;if (i>=1000)and(i<1800) then a[i]:=a[i]*100;if i=2800 then a[i]:=1e7;end;
 a:=agc.Process(a);Save('agc.f32',a);agc.Free;
 modul:=TModulator.Create;modul.SamplesPerSec:=11025;modul.CarrierFreq:=600;SetLengthReIm(iq,4096);
 for i:=0 to 4095 do begin iq.Re[i]:=Sin(i*0.013);iq.Im[i]:=Cos(i*0.023);end;
 a:=modul.Modulate(iq);Save('modulator.f32',a);modul.Free;
 RandomState:=144;fade:=TQsb.Create;fade.Bandwidth:=0.4;SetLength(a,512);SetLength(b,110*512);
 for j:=0 to 109 do begin for i:=0 to 511 do a[i]:=1;fade.ApplyTo(a);Move(a[0],b[j*512],512*4);end;
 Save('fading.f32',b);fade.Free;
 WriteLn('Reference fixtures generated from original Pascal units.');
end.
