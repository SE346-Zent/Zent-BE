import pandas as pd
import glob
import json
import os

files = glob.glob('data/*.xlsx')
print(f"Found {len(files)} files.")

for f in files:
    # Read the first sheet, we might need to skip rows. Let's read without header and search for the row with 'Part Number'
    df_raw = pd.read_excel(f, header=None)
    header_idx = -1
    for i, row in df_raw.iterrows():
        if 'Part Number' in row.values or 'Part Numer' in row.values: # User mentioned "Part Numer" so allow both
            header_idx = i
            break
            
    if header_idx != -1:
        df = pd.read_excel(f, header=header_idx)
        print(f"File {f} header found at row {header_idx}. Columns: {df.columns.tolist()[:10]}")
        # print some data
        # print(df.head(2).to_dict(orient='records'))
    else:
        print(f"Could not find header in {f}")
        for i, row in df_raw.head(10).iterrows():
            print(list(row.values[:5]))
            
    break
