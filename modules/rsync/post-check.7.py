import os
import datetime

def __get_day(index):
    trad = {
            0: 'Monday',
            1: 'Tuesday',
            2: 'Wednesday',
            3: 'Thurday',
            4: 'Friday',
            5: 'Saturday',
            6: 'Sunday'
    }

    return trad.get(index)

def get_day():
    return __get_day(datetime.datetime.now().weekday())

def is_mounted():
    handle = os.popen('ssh root@bach.dest.1 df')
    contents = handle.read()
    handle.close()
    pos = contents.find('/root/test2')
    return pos != -1

def list_files():
    handle = os.popen('ssh root@bach.dest.1 ls -l /root/test2')
    contents = handle.read()
    handle.close()
    print(contents)

if __name__ == '__main__':
    follow = True
    if is_mounted():
        os._exit(1)
    raw = os.system('ssh root@bach.dest.1 mount -o loop,offset=1048576 /root/test2.img /root/test2')
    stat = os.waitstatus_to_exitcode(raw)
    if stat != 0:
        print("Unable to mount /root/test2.img")
        follow = False
    list_files()
    if follow:
        raw = os.system('ssh root@bach.dest.1 diff -r /root/test2/{} /root/compare'.format(get_day()))
        stat = os.waitstatus_to_exitcode(raw)
        if stat < 0:
            stat = stat * -1

        os.system('ssh root@bach.dest.1 rm -r /root/test2/*')
        for i in range(0,7):
            os.system('ssh root@bach.dest.1 mkdir -p /root/test2/{}'.format(__get_day(i)))

        os.system('ssh root@bach.dest.1 umount /root/test2')

    os._exit(stat)
