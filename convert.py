import pandas as pd
import glob
import json
import os
import math

files = glob.glob('data/*.xlsx')

part_types_map = {}
installed_parts_by_model = {}

# We will map each filename (e.g. IdeaPadPro516AKP10.xlsx) to the correct model name used in PRODUCT_MODELS
model_names_mapping = {
    "IdeaPadPro516AKP10": "IdeaPad Pro 5 16AKP10",
    "Legion515IRX10": "Legion 5 15IRX10 - Type 83LY",
    "LOQ15AHP9": "LOQ 15AHP9",
    "ThinkBook16G9IPL": "ThinkBook 16 G9 IPL - Type 21UR",
    "ThinkPadX915p": "X9-15p Gen 1 - Type 21VV, 21VW",
    "Yoga92in114IPH11": "Yoga 9 2-in-1 14IPH11"
}

for f in files:
    filename = os.path.basename(f).replace('.xlsx', '')
    model_name = model_names_mapping.get(filename, filename)
    
    # Read without header
    df_raw = pd.read_excel(f, header=None)
    header_idx = -1
    for i, row in df_raw.iterrows():
        if 'Part Number' in row.values or 'Part Numer' in row.values:
            header_idx = i
            break
            
    if header_idx == -1:
        print(f"Skipping {f}, no 'Part Number' found.")
        continue
        
    df = pd.read_excel(f, header=header_idx)
    
    # Standardize column name
    if 'Part Numer' in df.columns:
        df.rename(columns={'Part Numer': 'Part Number'}, inplace=True)
        
    if 'Commodity Type' not in df.columns or 'Description' not in df.columns or 'Installed Qty' not in df.columns:
        print(f"Missing required columns in {f}: {df.columns.tolist()}")
        continue
        
    installed_parts_by_model[model_name] = []
    
    for _, row in df.iterrows():
        part_number = str(row['Part Number']).strip()
        commodity_type = str(row['Commodity Type']).strip()
        description = str(row['Description']).strip()
        qty = row['Installed Qty']
        if pd.isna(qty):
            qty = 1
        elif isinstance(qty, float) and math.isnan(qty):
            qty = 1
        else:
            try:
                qty = int(qty)
            except:
                qty = 1
                
        if part_number == 'nan' or not part_number:
            continue
            
        if part_number not in part_types_map:
            part_types_map[part_number] = {
                'part_number': part_number,
                'commodity_type': commodity_type,
                'description': description
            }
            
        installed_parts_by_model[model_name].append({
            'part_number': part_number,
            'quantity': qty
        })

output_dir = r"d:\Ryan\App_project\Zent\Zent-BE\seeder\resources"
os.makedirs(output_dir, exist_ok=True)
out_path = os.path.join(output_dir, "parts.json")

result = {
    "part_types": list(part_types_map.values()),
    "installations": installed_parts_by_model
}

with open(out_path, "w", encoding="utf-8") as out:
    json.dump(result, out, indent=2, ensure_ascii=False)

print(f"Written {len(result['part_types'])} part types and mapping for {len(installed_parts_by_model)} models to {out_path}")
