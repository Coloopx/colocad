from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
LABELS={'plate4': ['四孔安装板', '机械零件', '80 × 50 × 6 mm；4 个 Ø6 贯穿孔。'], 'plate64': ['64 孔阵列板', '重复特征', '80 × 80 × 6 mm；8 × 8 个 Ø4 孔，8 mm 间距。'], 'fillet': ['圆角结构件', '边缘特征', '60 × 40 × 20 mm；上缘四条边 R3 圆角。'], 'revolve': ['270° 回转件', '旋转特征', '同一五边截面绕 Z 轴回转 270°，保留开口。'], 'product': ['带盖电子设备外壳', '产品装配', '圆角薄壁壳、窗口、24 个通风孔、4 个盲孔柱、盖板、PCB 与屏幕包络；4 个实体。'], 'surface': ['可编辑自由曲面', '自由曲面', '同一 Bézier 控制网与 C2 桥接公式，随后改控制点并施加刚体变换；开放曲面，无实体体积。']}
def cases():return {k:(*v,json.loads((ROOT/'models'/(k+'.json')).read_text())) for k,v in LABELS.items()}
