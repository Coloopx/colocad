# Research-only empty container, no CAD objects or vendor part data.
import struct,zlib
from pathlib import Path
name='swXmlContents/Features'.encode();name=bytes(((i<<4)|(i>>4))&255 for i in name)
p=b'<swSolidWorks><swModel swName="Untitled" swConfigurationName="Default"/></swSolidWorks>'
z=zlib.compressobj(wbits=-15);packed=z.compress(p)+z.flush()
hdr=struct.pack('<HHHIIIIHH',20,6,8,0,zlib.crc32(p),len(packed),len(p),len(name),0)
# Signature words observed together in archived documents with this envelope dialect.
local=bytes.fromhex('59bb1573')+hdr+name+packed
central=bytes.fromhex('ba57fe3f0000')+hdr+bytes(14)+name
footer=bytes.fromhex('f295f2b9')+struct.pack('<HHHHIIH',0,0,1,1,len(central),len(local),0)
Path('empty-envelope.bin').write_bytes(bytes.fromhex('195dfb1000000004')+local+central+footer)
