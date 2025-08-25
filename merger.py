import pandas as pd

def append_dataframes(dfs: list):
    # Append two DataFrames
    appended = pd.concat(dfs, ignore_index=True)
    return appended

df1 = pd.read_csv("students_22_23.csv")
df2 = pd.read_csv("students_24.csv")

complete_df = append_dataframes([df1, df2])
complete_df.to_csv("students_complete.csv", index=False)