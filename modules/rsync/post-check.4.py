import os

if __name__ == '__main__':
    raw = os.system('ssh root@bach.dest.1 diff -r /root/compare /root/dummy-dat2')
    stat = os.waitstatus_to_exitcode(raw)
    if stat < 0:
        stat = stat *-1

    os.system('ssh root@bach.dest.1 rm -r /root/dummy-dat2/*')
    os._exit(stat)
