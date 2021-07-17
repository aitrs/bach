import os

def is_mounted():
    handle = os.popen('ssh root@bach.dest.1 df')
    contents = handle.read()
    handle.close()
    pos = contents.find('/root/test2')
    return pos != -1

if __name__ == '__main__':
    if is_mounted():
        os._exit(1)
    raw = os.system('ssh root@bach.dest.1 mount -o loop,offset=1048576 /root/test.img /root/test2')
    stat = os.waitstatus_to_exitcode(raw)
    if stat != 0:
        os._exit(1)

    raw = os.system('ssh root@bach.dest.1 diff -r /root/test2 /root/compare')
    stat = os.waitstatus_to_exitcode(raw)
    if stat < 0:
        stat = stat * -1
    os.system('ssh root@bach.dest.1 rm -r /root/test2/*');
    os.system('ssh root@bach.dest.1 umount /root/test2')

    os._exit(stat)
