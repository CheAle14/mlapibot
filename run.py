import os
print("hey")
os.chdir(os.path.join(os.getcwd(), "mlapi"))
print("ho")
import mlapi.main
print("Running...")
mlapi.main.start()