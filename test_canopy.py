import canopy
print(f"canopy version: {canopy.__version__}")
print(f"canopy dir: {dir(canopy)}")
if hasattr(canopy, "System"):
    print("canopy has System")
else:
    print("canopy DOES NOT have System")
if hasattr(canopy, "Query"):
    print("canopy has Query")
else:
    print("canopy DOES NOT have Query")
