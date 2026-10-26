from pathlib import Path
import subprocess,json,hashlib
base=Path('/Users/developer/Movies/meetily-recordings/Meeting 20_09_26_13_57_41_2026-09-20_10-58')
ff='/Users/developer/Desktop/meeting-minutes/.worktrees/feat071-audio-controls/target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-ui-error-fixed/meetily.app/Contents/MacOS/ffmpeg'
outdir=Path('/private/tmp/meetily-urgent-20260919/dts-pcm-prototype');outdir.mkdir(exist_ok=False);results=[]
for kind in ('microphone','system'):
 root=base/'.tracks'/kind;manifest=json.loads((root/'track.json').read_text());rate=manifest['sample_rate'];channels=manifest['channels'];output=outdir/(kind+'.mp4');encoderlog=outdir/(kind+'-encoder.log')
 with encoderlog.open('wb') as err:
  encoder=subprocess.Popen([ff,'-v','warning','-n','-f','f32le','-ar',str(rate),'-ac',str(channels),'-i','pipe:0','-c:a','aac','-b:a','192k','-movflags','+faststart',str(output)],stdin=subprocess.PIPE,stderr=err)
  pieces=[]
  for c in manifest['checkpoints']:
   p=root/'.checkpoints'/c['file_name'];assert hashlib.sha256(p.read_bytes()).hexdigest()==c['sha256'];expected=c['input_sample_count']*4;sent=0;decoded=0
   with (outdir/(kind+'-'+str(c['index'])+'-decoder.log')).open('wb') as derr:
    dec=subprocess.Popen([ff,'-v','warning','-i',str(p),'-map','0:a:0','-f','f32le','-ar',str(rate),'-ac',str(channels),'pipe:1'],stdout=subprocess.PIPE,stderr=derr)
    while True:
     block=dec.stdout.read(65536)
     if not block:break
     decoded+=len(block);take=min(len(block),expected-sent)
     if take:encoder.stdin.write(block[:take]);sent+=take
    code=dec.wait();assert code==0 and sent==expected,(kind,c['index'],code,sent,expected)
   pieces.append({'index':c['index'],'expected_samples':expected//4,'decoded_samples':decoded//4,'padding_trimmed_samples':(decoded-sent)//4})
  encoder.stdin.close();code=encoder.wait();assert code==0
 check=subprocess.run([ff,'-v','warning','-i',str(output),'-ar','8000','-f','null','-'],capture_output=True)
 info=subprocess.run(['/usr/local/bin/ffprobe','-v','error','-show_entries','stream=duration,sample_rate,channels','-of','json',str(output)],capture_output=True)
 results.append({'track':kind,'output':str(output),'expected_seconds':manifest['sample_count']/rate/channels,'pieces':pieces,'encoder_log':encoderlog.read_text(),'decode8k_exit':check.returncode,'decode8k_log':check.stderr.decode(),'probe':json.loads(info.stdout),'output_sha256':hashlib.sha256(output.read_bytes()).hexdigest()})
result={'scope':'temporary prototype; originals unchanged; not product implementation','method':'each checkpoint decoded separately, encoder padding trimmed to recorded input_sample_count; bounded 64KiB copy to single final AAC encoder','results':results}
(outdir/'result.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
