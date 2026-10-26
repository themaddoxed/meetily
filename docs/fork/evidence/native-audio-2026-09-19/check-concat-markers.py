from pathlib import Path
import array,math,subprocess,json,hashlib
root=Path('/private/tmp/meetily-urgent-20260919/dts-controlled-input');root.mkdir(exist_ok=False)
ff='/Users/developer/Desktop/meeting-minutes/.worktrees/feat071-audio-controls/target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-ui-error-fixed/meetily.app/Contents/MacOS/ffmpeg'
rate=48000;frames=[48013,48029,24117];b1=frames[0]/rate;b2=sum(frames[:2])/rate;starts=[b1-.07,b1+.03,b2-.07,b2+.03];duration=.03
for kind,ch in [('microphone',2),('system',1),('audio',1)]:
 d=root/'.tracks'/kind;parts=d/'.checkpoints';parts.mkdir(parents=True);entries=[];offset=0
 for i,n in enumerate(frames):
  data=array.array('f')
  for frame in range(n):
   t=(offset+frame)/rate;v=sum(.5*math.sin(2*math.pi*1200*(t-start)) for start in starts if start<=t<start+duration)
   data.extend([v]*ch)
  p=parts/f'{kind}_chunk_{i:03}.mp4';r=subprocess.run([ff,'-v','error','-n','-f','f32le','-ar',str(rate),'-ac',str(ch),'-i','pipe:0','-c:a','aac','-b:a','192k',str(p)],input=data.tobytes(),capture_output=True);assert r.returncode==0,r.stderr
  entries.append({'index':i,'file_name':p.name,'input_sample_count':n*ch,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()});offset+=n
 (d/'track.json').write_text(json.dumps({'sample_rate':rate,'channels':ch,'sample_count':sum(frames)*ch,'checkpoint_count':3,'checkpoints':entries}))
script=Path('/private/tmp/meetily-urgent-20260919/check-concat-pcm.py').read_text().replace("base=Path('/Users/developer/Movies/meetily-recordings/Meeting 20_09_26_13_57_41_2026-09-20_10-58')",f'base=Path({str(root)!r})').replace('dts-pcm-prototype','dts-controlled-output').replace("('microphone','system')","('microphone','system','audio')")
p=Path('/private/tmp/meetily-urgent-20260919/check-concat-controlled-run.py');p.write_text(script);subprocess.run(['python3',str(p)],check=True,stdout=subprocess.DEVNULL)
rows=[]
for kind in ['microphone','system','audio']:
 p=Path('/private/tmp/meetily-urgent-20260919/dts-controlled-output')/(kind+'.mp4');r=subprocess.run([ff,'-v','error','-i',str(p),'-ar',str(rate),'-ac','1','-f','f32le','pipe:1'],capture_output=True);assert r.returncode==0;data=array.array('f',r.stdout);bins=[]
 for k in range(0,len(data),240):
  chunk=data[k:k+240];rms=math.sqrt(sum(x*x for x in chunk)/len(chunk));bins.append((k/rate,rms>.1))
 segments=[]
 for t,on in bins:
  if on and (not segments or t>segments[-1][1]+.006):segments.append([t,t+.005])
  elif on:segments[-1][1]=t+.005
 assert len(segments)==4,(kind,segments)
 offsets=[actual[0]-expected for actual,expected in zip(segments,starts)];assert max(map(abs,offsets))<.01,(kind,offsets)
 rows.append({'track':kind,'markers_seconds':segments,'expected_starts':starts,'onset_errors_seconds':offsets,'decoded_frames':len(data),'expected_input_frames':sum(frames)})
out={'scope':'synthetic marker prototype only, not product implementation','checkpoint_frames':frames,'expected_marker_count':4,'onset_tolerance_seconds':.01,'results':rows}
Path('/private/tmp/meetily-urgent-20260919/dts-controlled-output/markers.json').write_text(json.dumps(out,indent=2));print(json.dumps(out,indent=2))
