import os

if __name__ == '__main__':
    raw = os.system('ssh root@bach.dest.1 diff -r /root/dummy-dat /root/compare')
    stat = os.waitstatus_to_exitcode(raw)

    if stat < 0:
        stat = stat * -1

    os.system('ssh root@bach.dest.1 rm -r /root/dummy-dat/*')
    os._exit(stat)
