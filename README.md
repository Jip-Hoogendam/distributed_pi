a project to calculuate pi on multiple servers at the same time. uses the chudnovsky approximation algorithem to approximate pi.

# computation time
# pc : 201.935080194 for 100 mil
# pc-chunked 6 clients 2 cores chunksize 100000 : 355.830511336 for 100 mil
# pc-chunked 1 client 16 cores chunksize 100000 : 373.990539372 for 100 mil
# pc-chunked 6 clients 4 cores chunksize 100000 : 352.47551185 for 100 mil
# pc-chunked 6 clients 4 cores chunksize 1000000 : 269.477002316 for 100 mil
# pc-chunked 5 clients 4 cores chunksize 1000000 : 107.464075386 for 100 mil //not to be compared to prev
# pc-chunked 5 clients 4 cores chunksize 400000 : 110.05791878 for 100 mil 

future improvements.

insted of using a diffent service to test for the digets of pi. use a spigot algorithem.
